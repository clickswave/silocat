use crate::models::{User, UserTokenData, token_data};
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::libs;

#[derive(Deserialize)]
pub struct GoogleAuthInput {
    pub code: String,
    // The frontend sends the exact redirect_uri it used (origin-derived), so this
    // works across dev / staging / prod. Google validates it against the URIs
    // registered in the OAuth client, so accepting it from the client is safe.
    pub redirect_uri: Option<String>,
    // ISO country from the user's CF-IPCountry (forwarded by the web proxy);
    // preferred over geoip since this call is proxied server-side.
    pub client_country: Option<String>,
}

/// Sanitize a 2-letter ISO country code; drops Cloudflare's non-country
/// sentinels (XX = unknown, T1 = Tor) and anything malformed.
fn sanitize_country(raw: Option<&str>) -> Option<String> {
    raw.map(|c| c.trim().to_uppercase()).filter(|c| {
        c.len() == 2 && c.chars().all(|ch| ch.is_ascii_alphabetic()) && c != "XX" && c != "T1"
    })
}

#[derive(Deserialize, Debug)]
pub struct GoogleTokenResponse {
    pub access_token: String,
    pub id_token: Option<String>, // present only when scope includes openid; unused (we use access_token).
    pub expires_in: i64,
    pub token_type: String,
    pub scope: String,
    // refresh_token...
}

#[derive(Deserialize, Debug)]
pub struct GoogleUserInfo {
    pub id: String,
    pub email: String,
    pub verified_email: bool,
    pub name: String,
    pub picture: String,
}

pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<GoogleAuthInput>,
) -> impl IntoResponse {
    let client_id = &axum_state.google_oauth.client_id;
    let client_secret = &axum_state.google_oauth.client_secret;
    // Use the redirect_uri the frontend used (must match the one in the auth
    // request and be registered in the Google OAuth client). Falls back to the
    // GOOGLE_REDIRECT_URI env var, then a local-dev default.
    let redirect_uri: String = payload
        .redirect_uri
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("GOOGLE_REDIRECT_URI").ok())
        .unwrap_or_else(|| "http://localhost:5173/auth/callback".to_string());
    let redirect_uri = redirect_uri.as_str();

    let client = reqwest::Client::new();

    // 1. Exchange code for token
    let params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", payload.code.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];

    let token_res: Result<reqwest::Response, reqwest::Error> = client.post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await;

    let token_res = match token_res {
        Ok(res) => res,
        Err(_e) => return respond(500, "Failed to connect to Google", vec![], json!({})),
    };

    if !token_res.status().is_success() {
         let error_text = token_res.text().await.unwrap_or_default();
         return respond(400, "Google Auth Failed", vec![error_text], json!({}));
    }

    let token_data: GoogleTokenResponse = match token_res.json().await {
        Ok(data) => data,
        Err(_e) => return respond(500, "Failed to parse Google response", vec![], json!({})),
    };

    // 2. Fetch User Info
    let user_info_res = client.get("https://www.googleapis.com/oauth2/v2/userinfo")
        .header("Authorization", format!("Bearer {}", token_data.access_token))
        .send()
        .await;

    let user_info_res = match user_info_res {
        Ok(res) => res,
        Err(_e) => return respond(500, "Failed to fetch Google profile", vec![], json!({})),
    };

    let google_user: GoogleUserInfo = match user_info_res.json().await {
        Ok(data) => data,
        Err(_e) => return respond(500, "Failed to parse Google profile", vec![], json!({})),
    };

    // Only trust a Google-verified email: otherwise an unverified Google email
    // could be used to log into / claim an existing account with that address.
    if !google_user.verified_email {
        return respond(
            400,
            "Email not verified",
            vec!["Your Google account's email address is not verified.".to_string()],
            json!({}),
        );
    }

    // 3. Find or Create User
    let existing_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
        .bind(&google_user.email)
        .fetch_optional(&axum_state.pg_pool)
        .await;

    match existing_user {
        Ok(Some(mut user)) => {
            // User exists, log them in.
            // If email verified on Google but not in DB, update it?
            if google_user.verified_email && !user.email_verified {
                let _ = sqlx::query!("UPDATE users SET email_verified = true, otp = '' WHERE id = $1", user.id)
                    .execute(&axum_state.pg_pool)
                    .await;
                user.email_verified = true;
            }
            
            // Return token data
            let token_data = crate::models::token_data(user, None); // TODO: fetch subscription? 
            // login.rs usually fetches sub.
            return respond(200, "Login successful", vec![], json!(token_data));
        },
        Ok(None) => {
            // First Google sign-in: create the account. Google already verified the
            // email; there is no password (login is via Google). Default 50GB storage.
            let base: String = google_user
                .name
                .to_lowercase()
                .chars()
                .filter(|c| c.is_ascii_alphanumeric())
                .collect();
            let base = if base.is_empty() {
                google_user.email.split('@').next().unwrap_or("user").to_string()
            } else {
                base
            };
            let username = format!("{}{}", base, libs::rng::number(4));
            let user_id = libs::rng::uuid();
            let api_key = libs::rng::uuid();
            let geo_country = sanitize_country(payload.client_country.as_deref()).or_else(|| {
                axum_state
                    .geoip
                    .as_ref()
                    .and_then(|r| libs::geoip::country_code(r, &libs::geoip::client_ip(&headers, addr)))
            });

            let created = sqlx::query_as::<_, User>(
                "INSERT INTO users \
                 (id, username, email, password_hash, otp, api_key, account_type, default_storage_bytes, email_verified, profile_image, country) \
                 VALUES ($1, $2, $3, '', '', $4, 'personal', $5, true, $6, $7) RETURNING *",
            )
            .bind(&user_id)
            .bind(&username)
            .bind(&google_user.email)
            .bind(&api_key)
            .bind(53687091200_i64)
            .bind(&google_user.picture)
            .bind(&geo_country)
            .fetch_one(&axum_state.pg_pool)
            .await;

            match created {
                Ok(user) => {
                    let token_data = crate::models::token_data(user, None);
                    return respond(200, "Signup successful", vec![], json!(token_data));
                }
                Err(_e) => {
                    return respond(500, "Could not create account", vec![], json!({}));
                }
            }
        },
        Err(_e) => {
             return respond(500, "Database error", vec![], json!({}));
        }
    }
}
