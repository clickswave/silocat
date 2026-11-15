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
    // First-touch `ref`, as on the email signup (migration 0047). Only used
    // when this sign-in creates the account.
    pub source: Option<String>,
}

/// Sanitize a 2-letter ISO country code; drops Cloudflare's non-country
/// sentinels (XX = unknown, T1 = Tor) and anything malformed.
fn sanitize_country(raw: Option<&str>) -> Option<String> {
    raw.map(|c| c.trim().to_uppercase()).filter(|c| {
        c.len() == 2 && c.chars().all(|ch| ch.is_ascii_alphabetic()) && c != "XX" && c != "T1"
    })
}

/// The username stem for an account a Google sign-in makes or claims: the
/// Google name in lowercase alphanumerics, or the email's local part.
fn username_base(google_user: &GoogleUserInfo) -> String {
    let base: String = google_user
        .name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    if base.is_empty() {
        google_user.email.split('@').next().unwrap_or("user").to_string()
    } else {
        base
    }
}

/// A username nobody has yet, for a first Google sign-in.
async fn acquire_free_username(pool: &sqlx::PgPool, base: &str) -> Result<String, sqlx::Error> {
    let mut conn = pool.acquire().await?;
    super::account_claim::free_username(&mut conn, base).await
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

/// What a Google sign-in answers for an account that already holds this
/// email: the status, message, errors and body handed to `respond`.
type Answer = (i32, &'static str, Vec<String>, serde_json::Value);

async fn sign_in_existing(st: &crate::AppState, google_user: &GoogleUserInfo, mut user: User) -> Answer {
    // Banned accounts get no session here either, the same refusal password
    // login gives. Checked before a claim so a refused sign-in changes nothing.
    let ban_active = user.is_banned && user.banned_until.map(|u| u > chrono::Utc::now()).unwrap_or(true);
    if ban_active {
        return (
            403,
            "Account banned",
            vec![user.ban_reason.clone().unwrap_or_else(|| "Your account has been banned.".to_string())],
            json!({ "banned": true, "reason": user.ban_reason, "until": user.banned_until }),
        );
    }

    // An unverified account was made by someone who never proved this
    // address, and Google has just shown the caller owns it. Take it
    // from the registrant before signing the caller in: new API key,
    // no password, new username, no share links. See account_claim.
    if !user.email_verified {
        use super::account_claim::{self, Claim};
        let key = match libs::apikey::mint() {
            Some(k) => k,
            None => return (500, "Server misconfigured", vec![], json!({})),
        };
        let base = username_base(google_user);
        let picture = Some(google_user.picture.as_str()).filter(|p| !p.is_empty());
        let claimed = async {
            let mut tx = st.pg_pool.begin().await?;
            let claim =
                account_claim::google_sign_in(&mut tx, &user.id, &user.email, &base, picture, &key).await?;
            tx.commit().await?;
            let row = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
                .bind(&user.id)
                .fetch_one(&st.pg_pool)
                .await?;
            Ok::<_, sqlx::Error>((claim, row))
        }
        .await;
        user = match claimed {
            Ok((Claim::Moved, _)) => {
                return (
                    409,
                    "Account changed",
                    vec!["This account changed while you were signing in. Please try again.".to_string()],
                    json!({}),
                );
            }
            Ok((claim, u)) => {
                if claim == Claim::TakenOver {
                    // The registrant's uploaded avatar, if any. The row
                    // no longer points at it.
                    let _ = st.r2.delete_object("dp", &u.id).await;
                }
                u
            }
            Err(e) => {
                eprintln!("[google_auth] claim failed: {:?}", e);
                return (500, "Database error", vec![], json!({}));
            }
        };
    }

    // Resolve the active subscription exactly as password login does.
    // Passing None here handed a paying user a free-tier session, so the
    // sidebar showed the wrong storage total until something else
    // refreshed it, and paid-only affordances read as locked.
    let subscription = sqlx::query_as!(
        crate::models::Subscription,
        "SELECT * FROM subscriptions WHERE id = $1 AND expires_on > NOW() \
         ORDER BY created_on DESC",
        user.subscription_id
    )
    .fetch_optional(&st.pg_pool)
    .await
    .unwrap_or(None);

    let token_data = crate::models::token_data(user, subscription);
    (200, "Login successful", vec![], json!(token_data))
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
        Ok(Some(user)) => {
            let (status, message, errors, data) = sign_in_existing(&axum_state, &google_user, user).await;
            return respond(status, message, errors, data);
        },
        Ok(None) => {
            // First Google sign-in: create the account. Google already verified the
            // email; there is no password (login is via Google). Storage comes from
            // libs::quota so it matches the password signup path exactly.
            let base = username_base(&google_user);
            let username = match acquire_free_username(&axum_state.pg_pool, &base).await {
                Ok(u) => u,
                Err(_e) => return respond(500, "Could not create account", vec![], json!({})),
            };
            let user_id = libs::rng::uuid();
            let minted = match libs::apikey::mint() {
                Some(k) => k,
                None => return respond(500, "Server misconfigured", vec![], json!({})),
            };
            let api_key = minted.blind_index.clone();
            let geo_country = sanitize_country(payload.client_country.as_deref()).or_else(|| {
                axum_state
                    .geoip
                    .as_ref()
                    .and_then(|r| libs::geoip::country_code(r, &libs::geoip::client_ip(&headers, addr)))
            });

            let created = sqlx::query_as::<_, User>(
                "INSERT INTO users \
                 (id, username, email, password_hash, otp, api_key, api_key_enc, api_key_migrated, account_type, default_storage_bytes, email_verified, profile_image, country, signup_source) \
                 VALUES ($1, $2, $3, '', '', $4, $5, TRUE, 'personal', $6, true, $7, $8, $9) RETURNING *",
            )
            .bind(&user_id)
            .bind(&username)
            .bind(&google_user.email)
            .bind(&api_key)
            .bind(&minted.encrypted)
            .bind(libs::quota::signup_storage_bytes())
            .bind(&google_user.picture)
            .bind(&geo_country)
            .bind(super::register_personal::sanitize_source(payload.source.as_deref()))
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

#[cfg(test)]
mod tests {
    //! These sign in to accounts in a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Each deletes the
    //! account it made.

    use super::*;
    use crate::test_support::{account, forget, state};

    async fn google_sign_in(st: &crate::AppState, id: &str) -> Answer {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        let google_user = GoogleUserInfo {
            id: "google-id".to_string(),
            email: user.email.clone(),
            verified_email: true,
            name: "Someone".to_string(),
            picture: String::new(),
        };
        sign_in_existing(st, &google_user, user).await
    }

    async fn set(st: &crate::AppState, sql: &str, id: &str) {
        sqlx::query(sql).bind(id).execute(&st.pg_pool).await.expect(sql);
    }

    #[tokio::test]
    async fn an_account_in_good_standing_signs_in() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        let (status, _, _, data) = google_sign_in(&st, &id).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 200);
        assert_eq!(data["id"], json!(id));
    }

    #[tokio::test]
    async fn a_banned_account_gets_no_session() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NULL, ban_reason = 'abuse' WHERE id = $1", &id)
            .await;
        let (status, _, _, data) = google_sign_in(&st, &id).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 403);
        assert!(data.get("api_key").is_none(), "a refused sign-in must not carry the key");
        assert_eq!(data["banned"], json!(true));
    }

    #[tokio::test]
    async fn a_temporarily_banned_account_gets_no_session_until_it_ends() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NOW() + INTERVAL '1 day' WHERE id = $1", &id).await;
        let (during, _, _, _) = google_sign_in(&st, &id).await;
        set(&st, "UPDATE users SET banned_until = NOW() - INTERVAL '1 minute' WHERE id = $1", &id).await;
        let (after, _, _, _) = google_sign_in(&st, &id).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(during, 403);
        assert_eq!(after, 200);
    }

    #[tokio::test]
    async fn a_banned_unverified_account_is_not_claimed() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, key_index) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET email_verified = FALSE, is_banned = TRUE WHERE id = $1", &id).await;
        let (status, _, _, _) = google_sign_in(&st, &id).await;
        let still: String = sqlx::query_scalar("SELECT api_key FROM users WHERE id = $1")
            .bind(&id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 403);
        assert_eq!(still, key_index, "a refused sign-in must leave the account as it was");
    }
}
