use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, routes::respond};

#[derive(Deserialize, Debug)]
pub struct ForgotPasswordInput {
    pub email: String,
}

/// POST /user/forgot-password  (public)
/// Generates a one-time code, stores it on the user, and emails it. Responds the
/// same way whether or not the email exists, so it can't be used to enumerate
/// accounts. Rate-limited to one code per 60s (shared with the verification OTP).
pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ForgotPasswordInput>,
) -> impl IntoResponse {
    // Per-IP throttle on reset-code sends (the per-account 60s throttle below
    // doesn't stop spraying codes across many accounts from one host).
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !state.rate_limiter.check(&format!("forgot:{}", ip), 10, std::time::Duration::from_secs(600)) {
        return respond(
            429,
            "Too Many Requests",
            vec!["Too many requests. Please try again later.".to_string()],
            json!({}),
        );
    }
    let generic_ok = || {
        respond(
            200,
            "If an account exists for that email, a reset code has been sent.",
            vec![],
            json!({}),
        )
    };

    let email = payload.email.trim().to_string();
    if email.is_empty() {
        return respond(400, "Email is required", vec![], json!({}));
    }

    // Look up the user; respond generically if not found. Runtime query (no
    // macro) keeps the SQLX_OFFLINE prod build cache-free.
    let row = sqlx::query_as::<_, (String, String, String, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT id, username, email, otp_last_sent_at FROM users WHERE email = $1",
    )
    .bind(&email)
    .fetch_optional(&state.pg_pool)
    .await;

    let (user_id, username, user_email, otp_last_sent_at) = match row {
        Ok(Some(u)) => u,
        Ok(None) => return generic_ok(),
        Err(e) => {
            println!("[FORGOT_PW] lookup failed: {:?}", e);
            return respond(500, "Something went wrong", vec![], json!({}));
        }
    };

    // Throttle: at most one code per 60s.
    if let Some(last_sent) = otp_last_sent_at {
        let diff = chrono::Utc::now().signed_duration_since(last_sent);
        if diff.num_seconds() < 60 {
            // Within the per-account cooldown: silently skip the resend and give
            // the SAME generic response as an unknown email. Returning a distinct
            // 429 only when the account exists is an account-enumeration oracle.
            return generic_ok();
        }
    }

    match issue_code(&state.pg_pool, &state.smtp_config, &user_id, &username, &user_email).await {
        Ok(()) => {}
        Err(Issue::Store(e)) => {
            println!("[FORGOT_PW] otp store failed: {:?}", e);
            return respond(500, "Something went wrong", vec![], json!({}));
        }
        Err(Issue::Send(e)) => {
            println!("[FORGOT_PW] email send failed: {}", e);
            return respond(500, "Could not send the reset email", vec![], json!({}));
        }
    }

    generic_ok()
}

enum Issue {
    Store(sqlx::Error),
    Send(String),
}

/// Send a new reset code, and only once it has gone out, make it the live one.
/// Storing first meant a send the mail limiter refused still replaced the code
/// the owner was sent and set its guess count back to zero.
async fn issue_code(
    pool: &sqlx::PgPool,
    smtp: &libs::configs::SmtpConfig,
    user_id: &str,
    username: &str,
    user_email: &str,
) -> Result<(), Issue> {
    let otp = libs::rng::number(6);
    libs::email::send_password_reset_email(smtp, username, user_email, &otp)
        .await
        .map_err(Issue::Send)?;
    sqlx::query("UPDATE users SET otp = $1, otp_last_sent_at = NOW(), otp_expires_at = NOW() + INTERVAL '10 minutes', otp_attempts = 0 WHERE id = $2")
        .bind(&otp)
        .bind(user_id)
        .execute(pool)
        .await
        .map_err(Issue::Store)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! Runs against a Postgres that has the api_switch migrations applied,
    //! named by SILOCAT_TEST_DATABASE_URL, and skips when it is unset. It needs
    //! a pool, so it deletes the account it made.

    use super::*;
    use crate::libs::{apikey, rng};

    fn smtp() -> libs::configs::SmtpConfig {
        libs::configs::SmtpConfig {
            address: "smtp.invalid".into(),
            username: String::new(),
            password: String::new(),
            from_name: "Silocat".into(),
            from_email: "team@silo.cat".into(),
            reply_to_name: "Silocat".into(),
            reply_to_email: "team@silo.cat".into(),
            use_ses_templates: false,
            ses_region: "us-east-1".into(),
            template_prefix: "silocat-".into(),
        }
    }

    #[tokio::test]
    async fn a_refused_send_leaves_the_code_and_its_count_alone() {
        let Ok(url) = std::env::var("SILOCAT_TEST_DATABASE_URL") else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        let pool = sqlx::PgPool::connect(&url).await.expect("connect");
        let id = rng::uuid();
        // example.com is never delivered to, so the send succeeds without
        // leaving the process, but it still goes through the mail limiter.
        let email = format!("{}@example.com", id);
        let key = apikey::mint().expect("test key");
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
             api_key_migrated, account_type, email_verified) \
             VALUES ($1, $2, $3, 'hash', '', $4, $5, TRUE, 'personal', true)",
        )
        .bind(&id)
        .bind(format!("reset{}", rng::string(10)).to_lowercase())
        .bind(&email)
        .bind(&key.blind_index)
        .bind(&key.encrypted)
        .execute(&pool)
        .await
        .expect("seed user");
        let read = |pool: sqlx::PgPool, id: String| async move {
            sqlx::query_as::<_, (String, i32)>("SELECT otp, otp_attempts FROM users WHERE id = $1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .expect("read otp")
        };

        let first = issue_code(&pool, &smtp(), &id, "owner", &email).await.is_ok();
        let _ = sqlx::query("UPDATE users SET otp_attempts = 3 WHERE id = $1").bind(&id).execute(&pool).await;
        let before = read(pool.clone(), id.clone()).await;
        // The mail limiter refuses a second reset email inside its cooldown.
        let second = issue_code(&pool, &smtp(), &id, "owner", &email).await.is_ok();
        let after = read(pool.clone(), id.clone()).await;

        let _ = sqlx::query("DELETE FROM users WHERE id = $1").bind(&id).execute(&pool).await;
        assert!(first, "the first reset email was refused");
        assert!(!before.0.is_empty(), "no code was stored");
        assert!(!second, "the second reset email was not refused");
        assert_eq!(after, before, "a code nobody was sent replaced the live one");
    }
}
