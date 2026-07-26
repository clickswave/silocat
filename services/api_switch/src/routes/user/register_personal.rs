use crate::libs::rng;
use crate::models::UserTokenData;
use crate::routes::respond;
use crate::{libs, models};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

#[derive(Deserialize, Debug)]
pub struct RegisterAccountInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub invite_code: Option<String>,
    pub promo_code: Option<String>,
    // ISO country code the web layer derived from the user's CF-IPCountry header.
    // Preferred over geoip here, because the IP this API sees is the web Worker's
    // (registration is proxied server-side), not the real client's.
    pub client_country: Option<String>,
}

/// Sanitize a 2-letter ISO country code; drops empties and Cloudflare's
/// non-country sentinels (XX = unknown, T1 = Tor).
fn sanitize_country(raw: Option<&str>) -> Option<String> {
    raw.map(|c| c.trim().to_uppercase()).filter(|c| {
        c.len() == 2 && c.chars().all(|ch| ch.is_ascii_alphabetic()) && c != "XX" && c != "T1"
    })
}

pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<RegisterAccountInput>,
) -> impl IntoResponse {
    // Default the new user's country from the IP they register from (best-effort;
    // None for unresolvable/private IPs, e.g. local dev).
    let registration_ip = libs::geoip::client_ip(&headers, addr);
    // Per-IP throttle on account creation to blunt mass signup abuse.
    if !axum_state.rate_limiter.check(&format!("register:{}", registration_ip), 5, std::time::Duration::from_secs(600)) {
        return respond(
            429,
            "Too Many Requests",
            vec!["Too many accounts created from your network. Please try again later.".to_string()],
            json!({}),
        );
    }
    let geo_country = axum_state
        .geoip
        .as_ref()
        .and_then(|r| libs::geoip::country_code(r, &registration_ip));
    // Prefer the client's real country (from the web's CF-IPCountry); geoip on the
    // Worker IP is the fallback (and is right for direct/dev hits).
    let geo_country = sanitize_country(payload.client_country.as_deref()).or(geo_country);
    // input validation
    let mut validation_errors = vec![];
    // validate email
    if let Err(errors) = libs::input_validators::email(&payload.email) {
        validation_errors.extend(errors);
    }
    // validate password
    if let Err(error) = libs::input_validators::password(&payload.password) {
        validation_errors.extend(error);
    }
    // return all the validation errors
    if validation_errors.len() > 0 {
        return respond(
            400,
            "Your input contains errors",
            validation_errors,
            json!({}),
        );
    }

    // validate invite code
    let valid_invite_code = match payload.invite_code {
        Some(ref code) => {
            let invite_code_exists = sqlx::query_as!(
                models::InviteCode,
                "
                SELECT * FROM invite_codes WHERE code = $1 AND account_type = 'personal' AND claimed_by IS NULL
                ",
                code
            ).fetch_optional(&axum_state.pg_pool).await;

            match invite_code_exists {
                Ok(invite_code) => invite_code,
                Err(_) => {
                    return respond(
                        500,
                        "Could not create a new account",
                        vec![
                            "Something went wrong while trying to verify your invite code"
                                .to_string(),
                        ],
                        json!({}),
                    );
                }
            }
        }
        None => None,
    };

    // Signup gate. Invite-only by default; set SILOCAT_INVITE_ONLY=false to open
    // public registration. Invites still grant their benefits when supplied: // they just stop being mandatory once signup is public.
    // Public signup by default now (promo codes replaced the invite requirement).
    // Set SILOCAT_INVITE_ONLY=true to re-gate signup behind an invite code.
    let invite_only = std::env::var("SILOCAT_INVITE_ONLY")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "true" || v == "1" || v == "yes"
        })
        .unwrap_or(false);

    if invite_only && valid_invite_code.is_none() {
        return respond(
            400,
            "Invite code required",
            vec!["This service is currently invite-only. Please provide a valid invite code.".to_string()],
            json!({}),
        );
    }

    // hash the password
    let password_hash = match libs::argon2::hash(payload.password) {
        Ok(hash) => hash,
        Err(_) => {
            return respond(
                500,
                "Could not create a new account",
                vec!["Something went wrong while trying to hash your password".to_string()],
                json!({}),
            );
        }
    };

    let otp = libs::rng::number(6);
    let user_id = rng::uuid();


    // Determine initial storage. Free tier is 10 GB (existing users keep whatever
    // their row already holds: this default only applies to new signups).
    let mut initial_storage_bytes: i64 = 10 * 1024 * 1024 * 1024; // 10 GB default

    if let Some(ref invite) = valid_invite_code {
        if invite.benefit.ends_with("GB") {
            let gb_str = invite.benefit.trim_end_matches("GB");
            if let Ok(gb) = gb_str.parse::<i64>() {
                initial_storage_bytes = gb * 1024 * 1024 * 1024;
            }
        }
    }

    let minted = match libs::apikey::mint() {
        Some(k) => k,
        None => return respond(500, "Server misconfigured", vec![], json!({})),
    };

    let user = sqlx::query_as::<_, models::User>(
        "
        INSERT INTO users
            (
             id,
             username,
             email,
             password_hash,
             otp,
             otp_expires_at,
             api_key,
             api_key_enc,
             api_key_migrated,
             team_id,
             subscription_id,
             account_type,
             default_storage_bytes,
             country
            )
        VALUES
            ( $1, $2, $3, $4, $5, NOW() + INTERVAL '10 minutes', $6, $7, TRUE, $8, $9, $10, $11, $12)
        RETURNING *
        "
    )
    .bind(&user_id)
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(password_hash)
    .bind(&otp)
    .bind(&minted.blind_index)
    .bind(&minted.encrypted)
    .bind(None::<String>)
    .bind(None::<String>)
    .bind("personal")
    .bind(initial_storage_bytes)
    .bind(&geo_country)
    .fetch_one(&axum_state.pg_pool)
    .await;

    match user {
        Ok(user) => {
            // The subscription carried on the session token (drives the sidebar's
            // initial storage total before the live stats fetch resolves). A promo
            // OR an invite Pro grant fills this in.
            let mut created_subscription: Option<models::Subscription> = None;

            // Redeem an optional signup promo: grants bonus storage for a duration
            // (recorded as a subscription so fetch_storage_stats counts it until it
            // expires). Indefinite = ~100 years. Best-effort: never fails signup.
            if let Some(ref pc) = payload.promo_code {
                let code = pc.trim();
                if !code.is_empty() {
                    let promo = sqlx::query_as::<_, (i64, Option<i32>, Option<i32>, i32)>(
                        "SELECT bonus_bytes, duration_days, max_uses, uses_count \
                         FROM signup_promos WHERE code = $1 AND active = TRUE",
                    )
                    .bind(code)
                    .fetch_optional(&axum_state.pg_pool)
                    .await
                    .ok()
                    .flatten();

                    if let Some((bonus, duration_days, max_uses, uses_count)) = promo {
                        let has_room = max_uses.map(|m| uses_count < m).unwrap_or(true);
                        if has_room && bonus > 0 {
                            // duration_days comes from our own DB as an integer, safe to inline
                            let days = duration_days.unwrap_or(36500).max(0);
                            let insert_sub = format!(
                                "INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) \
                                 VALUES ('Promo', $1, $2, NOW() + INTERVAL '{} days', TRUE) RETURNING *",
                                days
                            );
                            // Only bump uses_count if the subscription row was actually
                            // created: otherwise the admin panel shows a redemption that
                            // never granted any space (the bug we are fixing).
                            match sqlx::query(&insert_sub)
                                .bind(bonus)
                                .bind(&user.id)
                                .fetch_optional(&axum_state.pg_pool)
                                .await
                            {
                                Ok(Some(row)) => {
                                    created_subscription = Some(models::Subscription {
                                        id: row.get("id"),
                                        name: row.get("name"),
                                        additional_space: row.get("additional_space"),
                                        created_by: row.get("created_by"),
                                        created_on: row.get("created_on"),
                                        expires_on: row.get("expires_on"),
                                        invited: row.get("invited"),
                                    });
                                    let _ = sqlx::query("UPDATE signup_promos SET uses_count = uses_count + 1 WHERE code = $1")
                                        .bind(code)
                                        .execute(&axum_state.pg_pool)
                                        .await;
                                }
                                Ok(None) => {}
                                Err(e) => {
                                    println!("Promo subscription insert failed for {}: {:?}", code, e);
                                }
                            }
                        }
                    }
                }
            }

            // Apply Subscription Benefits
            if let Some(ref invite) = valid_invite_code {
                let duration_days = match invite.benefit.as_str() {
                    "1m-pro" => Some(30),
                    "3m-pro" => Some(90),
                    "6m-pro" => Some(180),
                    "1y-pro" => Some(365),
                    _ => None
                };

                if let Some(days) = duration_days {
                     // Create Pro Subscription
                    let expires_on = Utc::now() + chrono::Duration::days(days);
                    
                    // We use sqlx::query (not macro) to match the surrounding style for dynamic inserts without macro overhead if DB not present
                    let sub_res = sqlx::query("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) VALUES ('Pro', 1099511627776, $1, $2, TRUE) RETURNING *")
                        .bind(&user.id)
                        .bind(expires_on)
                        .fetch_optional(&axum_state.pg_pool)
                        .await;

                   if let Ok(Some(row)) = sub_res {
                        // We fetch as dynamic row then map manually or use query_as if struct allows. 
                        // Ideally we cast to Subscription model if possible, but Row is fine for ID.
                        
                        let sub_id: String = row.get("id");
                        
                        // Update Token Data Subscription
                        // We can construct the Subscription object from the row to return it
                        let subscription = models::Subscription {
                            id: sub_id.clone(),
                            name: row.get("name"),
                            additional_space: row.get("additional_space"),
                            created_by: row.get("created_by"),
                            created_on: row.get("created_on"),
                            expires_on: row.get("expires_on"),
                            invited: row.get("invited"),
                        };
                        
                        // Update User Record
                        let _ = sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", sub_id, user.id)
                            .execute(&axum_state.pg_pool)
                            .await;
                            
                        // Store explicitly for token
                        created_subscription = Some(subscription);
                   }
                }
            }

            // send verification email
            if let Err(e) = libs::email::send_verification_email(
                &axum_state.smtp_config,
                &payload.username,
                &payload.email,
                &otp
            ).await {
                println!("Failed to send verification email: {}", e);
                // We typically don't fail the request here, but logging is essential.
            }

            // token struct
            let mut user_token_data = UserTokenData {
                id: user.id,
                username: user.username,
                email: user.email,
                email_verified: user.email_verified,
                profile_image: user.profile_image,
                password_set: true,
                subscription: created_subscription,
                api_key: user.api_key,
                account_type: user.account_type,
                default_storage_bytes: user.default_storage_bytes,
                country: user.country,
                bio: user.bio,
            };

            let code = match valid_invite_code {
                Some(invite_code) => invite_code.code,
                None => "invalid".to_string(),
            };

            // mark the invite code as claimed
            let _ = sqlx::query!(
                "
                UPDATE invite_codes SET claimed_by = $1 WHERE code = $2 AND claimed_by IS NULL
                ",
                user_id,
                code
            )
            .execute(&axum_state.pg_pool)
            .await;

            return respond(
                201,
                "Account registration was successful",
                vec![],
                json!({"user": user_token_data}),
            );
        }
        Err(error) => {
            let check_error = error.as_database_error();
            match check_error {
                None => {
                    return respond(
                        500,
                        "Could not create a new account",
                        vec!["Something went wrong while creating a new account".to_string()],
                        json!({}),
                    );
                }
                Some(db_error) => {
                    if db_error.is_unique_violation() {
                        return respond(
                            409,
                            "Could not create a new account",
                            vec![
                                "Username or email already exists, please choose a unique one"
                                    .to_string(),
                            ],
                            json!({}),
                        );
                    }
                }
            }

            if check_error.is_some() {
                if check_error.unwrap().is_unique_violation() {
                    return respond(
                        409,
                        "Could not create a new account",
                        vec!["User already exists, please choose a different email".to_string()],
                        json!({}),
                    );
                }
            }

            respond(
                500,
                "Could not create a new account",
                vec!["Something went wrong while creating a new account".to_string()],
                json!({}),
            )
        }
    };

    respond(
        500,
        "Could not create a new account",
        vec!["Something went wrong while creating a new account".to_string()],
        json!({}),
    )
}
