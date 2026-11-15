use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, routes::respond};

#[derive(Deserialize, Debug)]
pub struct ForgotPasswordInput {
    pub email: String,
}

/// POST /user/forgot-password  (public)
/// Emails the account a one-time reset code. Responds the same way whether or
/// not the email exists, so it can't be used to enumerate accounts. At most one
/// code per account per 60s, kept apart from the verification code's cooldown.
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

    let email = payload.email.trim().to_string();
    if email.is_empty() {
        return respond(400, "Email is required", vec![], json!({}));
    }

    match request_reset(&state.pg_pool, &state.smtp_config, &email).await {
        Ok(()) => respond(
            200,
            "If an account exists for that email, a reset code has been sent.",
            vec![],
            json!({}),
        ),
        Err(e) => {
            println!("[FORGOT_PW] {:?}", e);
            respond(500, "Something went wrong", vec![], json!({}))
        }
    }
}

/// Email a new reset code to the account at `email`, if there is one and it has
/// not had one in the last 60 seconds. Unknown addresses, the cooldown and a
/// send the mail limiter refuses all return Ok, so the caller answers them
/// alike; only a database error does not. The code becomes the live one only
/// after its email has gone out, so a refused send leaves the code the owner
/// already has, and its attempt count, as they were.
async fn request_reset(
    pool: &sqlx::PgPool,
    smtp: &libs::configs::SmtpConfig,
    email: &str,
) -> Result<(), sqlx::Error> {
    // Claiming the minute and finding the account are one statement, so two
    // requests at once cannot both send. Runtime query (no macro) keeps the
    // SQLX_OFFLINE prod build cache-free.
    let claimed = sqlx::query_as::<_, (String, String, String)>(
        "UPDATE users SET reset_last_sent_at = NOW() WHERE email = $1 \
         AND (reset_last_sent_at IS NULL OR reset_last_sent_at <= NOW() - INTERVAL '60 seconds') \
         RETURNING id, username, email",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;
    let Some((user_id, username, user_email)) = claimed else {
        return Ok(());
    };

    let code = libs::otp::new_reset_code();
    if let Err(e) = libs::email::send_password_reset_email(smtp, &username, &user_email, &code).await {
        println!("[FORGOT_PW] email send refused or failed: {}", e);
        return Ok(());
    }
    libs::otp::store_reset(pool, &user_id, &code).await
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
    async fn a_refused_send_answers_like_an_unknown_address_and_keeps_the_code() {
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
            sqlx::query_as::<_, (String, i32)>("SELECT reset_code, reset_attempts FROM users WHERE id = $1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .expect("read reset code")
        };

        let unknown = request_reset(&pool, &smtp(), &format!("nobody-{}@example.com", id)).await.is_ok();
        let first = request_reset(&pool, &smtp(), &email).await.is_ok();
        // A minute later as far as the account is concerned, with three wrong
        // guesses on the code, but the mail limiter refuses a second reset email.
        let _ = sqlx::query("UPDATE users SET reset_attempts = 3, reset_last_sent_at = NULL WHERE id = $1")
            .bind(&id)
            .execute(&pool)
            .await;
        let before = read(pool.clone(), id.clone()).await;
        let second = request_reset(&pool, &smtp(), &email).await.is_ok();
        let after = read(pool.clone(), id.clone()).await;

        let _ = sqlx::query("DELETE FROM users WHERE id = $1").bind(&id).execute(&pool).await;
        assert!(unknown && first, "an unknown address or the first send was answered with an error");
        assert!(!before.0.is_empty(), "no code was stored");
        assert!(second, "a refused send was answered unlike an unknown address");
        assert_eq!(after, before, "a code nobody was sent replaced the live one");
    }
}
