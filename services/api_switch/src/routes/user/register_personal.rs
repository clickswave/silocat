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
    // The first `ref` this visitor arrived with (see migration 0047). Optional;
    // cleaned by `sanitize_source` before it is stored.
    pub source: Option<String>,
}

/// Clean a client-supplied signup source: lowercase, only [a-z0-9_-], at most
/// 40 characters. Anything that comes out empty is dropped.
pub fn sanitize_source(raw: Option<&str>) -> Option<String> {
    let s: String = raw?
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(40)
        .collect();
    (!s.is_empty()).then_some(s)
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


    // Claim the invite before anything it grants is written. The check above
    // only read it, and parallel signups with one code all pass that check, so
    // only the signup whose conditional claim lands gets the benefit. The claim
    // is given back if the account is not created.
    let valid_invite_code = match valid_invite_code {
        Some(invite) => match sqlx::query_as::<_, models::InviteCode>(
            "UPDATE invite_codes SET claimed_by = $1 WHERE code = $2 AND claimed_by IS NULL RETURNING *",
        )
        .bind(&user_id)
        .bind(&invite.code)
        .fetch_optional(&axum_state.pg_pool)
        .await
        {
            Ok(claimed) => claimed,
            Err(_) => {
                return respond(
                    500,
                    "Could not create a new account",
                    vec!["Something went wrong while trying to verify your invite code".to_string()],
                    json!({}),
                );
            }
        },
        None => None,
    };
    if invite_only && valid_invite_code.is_none() {
        return respond(
            400,
            "Invite code required",
            vec!["This service is currently invite-only. Please provide a valid invite code.".to_string()],
            json!({}),
        );
    }

    // Determine initial storage. The default is owned by libs::quota so that this
    // path and the Google signup path cannot drift apart again (existing users keep
    // whatever their row already holds: this only applies to new signups).
    let mut initial_storage_bytes: i64 = libs::quota::signup_storage_bytes();

    if let Some(ref invite) = valid_invite_code {
        if invite.benefit.ends_with("GB") {
            let gb_str = invite.benefit.trim_end_matches("GB");
            if let Ok(gb) = gb_str.parse::<i64>() {
                initial_storage_bytes = gb.saturating_mul(libs::quota::GIB);
            }
        }
    }

    let minted = match libs::apikey::mint() {
        Some(k) => k,
        None => {
            release_invite(&axum_state.pg_pool, &valid_invite_code, &user_id).await;
            return respond(500, "Server misconfigured", vec![], json!({}));
        }
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
             country,
             signup_source
            )
        VALUES
            ( $1, $2, $3, $4, $5, NOW() + INTERVAL '10 minutes', $6, $7, TRUE, $8, $9, $10, $11, $12, $13)
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
    .bind(sanitize_source(payload.source.as_deref()))
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
                    // Take a use first, in one conditional statement, so parallel
                    // signups cannot all pass the cap check before any of them
                    // counts. The use is given back if the grant is not written.
                    let promo = sqlx::query_as::<_, (i64, Option<i32>)>(
                        "UPDATE signup_promos SET uses_count = uses_count + 1 \
                         WHERE code = $1 AND active = TRUE AND bonus_bytes > 0 \
                           AND (max_uses IS NULL OR uses_count < max_uses) \
                         RETURNING bonus_bytes, duration_days",
                    )
                    .bind(code)
                    .fetch_optional(&axum_state.pg_pool)
                    .await
                    .ok()
                    .flatten();

                    if let Some((bonus, duration_days)) = promo {
                        // duration_days comes from our own DB as an integer, safe to inline
                        let days = duration_days.unwrap_or(36500).max(0);
                        let insert_sub = format!(
                            "INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) \
                             VALUES ('Promo', $1, $2, NOW() + INTERVAL '{} days', TRUE) RETURNING *",
                            days
                        );
                        // A use is only kept when the subscription row was actually
                        // created: otherwise the admin panel shows a redemption that
                        // never granted any space.
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
                                    razorpay_subscription_id: None,
                                });
                            }
                            other => {
                                if let Err(e) = other {
                                    println!("Promo subscription insert failed for {}: {:?}", code, e);
                                }
                                let _ = sqlx::query(
                                    "UPDATE signup_promos SET uses_count = GREATEST(uses_count - 1, 0) WHERE code = $1",
                                )
                                .bind(code)
                                .execute(&axum_state.pg_pool)
                                .await;
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
                    // Canonical Pro grant. This used to hardcode 1 TiB
                    // (1099511627776) while billing Pro grants 2 TB; libs::plans is
                    // now the single source so invited and paid Pro match.
                    let (pro_name, pro_bytes) = crate::libs::plans::plan_grant("pro")
                        .unwrap_or(("Pro", 2 * crate::libs::plans::TIB));
                    let sub_res = sqlx::query("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) VALUES ($1, $2, $3, $4, TRUE) RETURNING *")
                        .bind(pro_name)
                        .bind(pro_bytes)
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
                            razorpay_subscription_id: None,
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
                // The session carries the real key so the client can send
                // X-Api-Key. user.api_key is the blind index, which authenticates
                // nothing: it is what the index of a presented key is compared to.
                api_key: user
                    .api_key_enc
                    .as_deref()
                    .and_then(crate::libs::apikey::decrypt)
                    .unwrap_or_default(),
                account_type: user.account_type,
                default_storage_bytes: user.default_storage_bytes,
                country: user.country,
                bio: user.bio,
            };

            return respond(
                201,
                "Account registration was successful",
                vec![],
                json!({"user": user_token_data}),
            );
        }
        Err(error) => {
            release_invite(&axum_state.pg_pool, &valid_invite_code, &user_id).await;
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

/// Give back an invite claimed for an account that was then not created.
async fn release_invite(pool: &sqlx::Pool<sqlx::Postgres>, invite: &Option<models::InviteCode>, user_id: &str) {
    if let Some(invite) = invite {
        let _ = sqlx::query("UPDATE invite_codes SET claimed_by = NULL WHERE code = $1 AND claimed_by = $2")
            .bind(&invite.code)
            .bind(user_id)
            .execute(pool)
            .await;
    }
}

#[cfg(test)]
mod tests {
    //! These register accounts in a Postgres named by SILOCAT_TEST_DATABASE_URL
    //! and skip when it is unset. Each deletes the accounts and codes it made.

    use super::*;
    use crate::test_support::{forget, state};
    use axum::extract::ConnectInfo;

    /// Several people sign up at once with the same code, each from its own
    /// address. Returns the ids of the accounts that were created.
    async fn signups(st: &crate::AppState, invite: Option<&str>, promo: Option<&str>) -> Vec<String> {
        let run = crate::libs::rng::string(8).to_lowercase();
        let mut tasks = Vec::new();
        for i in 0..6 {
            let st = st.clone();
            let payload = RegisterAccountInput {
                username: format!("race{}{}", run, i),
                email: format!("race{}{}@example.com", run, i),
                password: "Race-Password-42".to_string(),
                invite_code: invite.map(str::to_string),
                promo_code: promo.map(str::to_string),
                client_country: None,
                source: None,
            };
            tasks.push(tokio::spawn(async move {
                let mut headers = axum::http::HeaderMap::new();
                headers.insert("x-client-ip", format!("198.51.100.{}", i + 1).parse().unwrap());
                let resp = handle(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st), headers, Json(payload))
                    .await
                    .into_response();
                let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
                let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
                body.pointer("/data/user/id").and_then(|v| v.as_str()).map(str::to_string)
            }));
        }
        let mut ids = Vec::new();
        for t in tasks {
            if let Some(id) = t.await.unwrap() {
                ids.push(id);
            }
        }
        ids
    }

    async fn grants(st: &crate::AppState, ids: &[String], name: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM subscriptions WHERE created_by = ANY($1) AND name = $2")
            .bind(ids)
            .bind(name)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn forget_all(st: &crate::AppState, ids: &[String]) {
        for id in ids {
            forget(&st.pg_pool, id).await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn parallel_signups_get_a_single_use_invite_once() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let code = format!("INV{}", crate::libs::rng::string(10)).to_uppercase();
        sqlx::query("INSERT INTO invite_codes (code, account_type, benefit) VALUES ($1, 'personal', '1y-pro')")
            .bind(&code)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let ids = signups(&st, Some(&code), None).await;
        let pro = grants(&st, &ids, "Pro").await;
        let claimed_by: Option<String> = sqlx::query_scalar("SELECT claimed_by FROM invite_codes WHERE code = $1")
            .bind(&code)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget_all(&st, &ids).await;
        sqlx::query("DELETE FROM invite_codes WHERE code = $1").bind(&code).execute(&st.pg_pool).await.unwrap();
        assert_eq!(ids.len(), 6, "public signup still lets everyone in");
        assert_eq!(pro, 1, "one invite gives one Pro grant");
        assert!(claimed_by.is_some_and(|c| ids.contains(&c)), "the invite is claimed by an account that exists");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn parallel_signups_get_a_storage_invite_once() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let code = format!("INV{}", crate::libs::rng::string(10)).to_uppercase();
        sqlx::query("INSERT INTO invite_codes (code, account_type, benefit) VALUES ($1, 'personal', '5000GB')")
            .bind(&code)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let ids = signups(&st, Some(&code), None).await;
        let boosted: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE id = ANY($1) AND default_storage_bytes = $2")
            .bind(&ids)
            .bind(5000 * libs::quota::GIB)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget_all(&st, &ids).await;
        sqlx::query("DELETE FROM invite_codes WHERE code = $1").bind(&code).execute(&st.pg_pool).await.unwrap();
        assert_eq!(ids.len(), 6);
        assert_eq!(boosted, 1, "one invite gives its storage to one account");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn parallel_signups_stay_within_a_promo_cap() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let code = format!("SP{}", crate::libs::rng::string(10)).to_uppercase();
        sqlx::query("INSERT INTO signup_promos (code, bonus_bytes, max_uses, active) VALUES ($1, $2, 1, TRUE)")
            .bind(&code)
            .bind(100 * libs::quota::GIB)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        // Hold the code's row until all six signups are waiting on it, so they
        // all reach the redemption together.
        let mut hold = st.pg_pool.begin().await.unwrap();
        sqlx::query("SELECT 1 FROM signup_promos WHERE code = $1 FOR UPDATE")
            .bind(&code)
            .execute(&mut *hold)
            .await
            .unwrap();
        let racing = {
            let (st, code) = (st.clone(), code.clone());
            tokio::spawn(async move { signups(&st, None, Some(&code)).await })
        };
        for _ in 0..600 {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_activity WHERE datname = current_database() AND wait_event_type = 'Lock'",
            )
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
            if waiting >= 6 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        hold.commit().await.unwrap();
        let ids = racing.await.unwrap();
        let promo = grants(&st, &ids, "Promo").await;
        let used: i32 = sqlx::query_scalar("SELECT uses_count FROM signup_promos WHERE code = $1")
            .bind(&code)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget_all(&st, &ids).await;
        sqlx::query("DELETE FROM signup_promos WHERE code = $1").bind(&code).execute(&st.pg_pool).await.unwrap();
        assert_eq!(ids.len(), 6);
        assert_eq!(promo, 1, "a promo capped at one use grants once");
        assert_eq!(used, 1);
    }
}
