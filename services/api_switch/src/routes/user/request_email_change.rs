use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, models::UserTokenData, routes::respond};

#[derive(Deserialize)]
pub struct Input {
    pub email: String,
    // Required when the account has a password. Whoever holds the account's
    // API key or a session cookie must not be able to move the account to an
    // address they read, reset the password there and lock the owner out.
    #[serde(default)]
    pub current_password: Option<String>,
}

// Stage a new email + OTP without touching the live (verified) account email.
// Also used as the "resend" action: calling again regenerates + resends the code.
pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Extension(user): Extension<UserTokenData>,
    Json(payload): Json<Input>,
) -> impl IntoResponse {
    // Throttle before doing anything: this route emails an attacker-supplied
    // address, so without a limit one account can blast verification mail at any
    // inbox (SES-reputation drain) and spin fresh OTPs for a brute-force. Mirror
    // forgot-password: a per-IP cap plus a 60s per-account cooldown.
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !state.rate_limiter.check(
        &format!("emailchange:ip:{}", ip), 10, std::time::Duration::from_secs(600))
    {
        return respond(429, "Too Many Requests",
            vec!["Too many requests. Please try again later.".to_string()], json!({}));
    }
    if !state.rate_limiter.check(
        &format!("emailchange:req:{}", user.id), 1, std::time::Duration::from_secs(60))
    {
        return respond(429, "Too Many Requests",
            vec!["Please wait a minute before requesting another code.".to_string()], json!({}));
    }

    let new_email = payload.email.trim().to_lowercase();

    if new_email.is_empty() {
        return respond(400, "Email required", vec![], json!({}));
    }
    if let Err(errors) = libs::input_validators::email(&new_email) {
        return respond(400, "Invalid email", errors, json!({}));
    }
    if new_email == user.email.to_lowercase() {
        return respond(400, "Same email", vec!["That is already your email.".to_string()], json!({}));
    }

    let otp = libs::rng::number(6);
    let staged = match state.pg_pool.acquire().await {
        Ok(mut conn) => {
            stage(&mut conn, &user.id, payload.current_password.as_deref(), &new_email, &otp).await
        }
        Err(e) => Err(e),
    };
    match staged {
        Ok(Stage::Staged) => {}
        Ok(Stage::WrongPassword) => {
            return respond(
                401,
                "Authentication failed",
                vec!["Current password is incorrect".to_string()],
                json!({}),
            )
        }
        Ok(Stage::Taken) => {
            return respond(
                409,
                "Email already in use",
                vec!["That email is already registered to another account.".to_string()],
                json!({}),
            )
        }
        Err(_e) => return respond(500, "Failed to start email change", vec![], json!({})),
    }

    if let Err(e) =
        libs::email::send_verification_email(&state.smtp_config, &user.username, &new_email, &otp).await
    {
        println!("Failed to send email-change code: {}", e);
        return respond(500, "Could not send verification email", vec![], json!({}));
    }

    respond(200, "Verification code sent", vec![], json!({ "pending_email": new_email }))
}

#[derive(Debug, PartialEq)]
pub(super) enum Stage {
    /// `new_email` and `otp` are now the account's pending change.
    Staged,
    /// The account has a password and `current_password` is missing or wrong,
    /// or the account is gone. Nothing was written.
    WrongPassword,
    /// Another account owns `new_email`. Nothing was written.
    Taken,
}

/// Stage `new_email` with `otp` as the account's pending email change. An
/// account with a password has to supply it: the change ends with the new
/// address verified, and a password reset sent there is then a full takeover.
/// An account that has never set one (Google sign-up) has nothing to supply.
pub(super) async fn stage(
    conn: &mut sqlx::PgConnection,
    user_id: &str,
    current_password: Option<&str>,
    new_email: &str,
    otp: &str,
) -> Result<Stage, sqlx::Error> {
    let hash: Option<String> = sqlx::query_scalar("SELECT password_hash FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&mut *conn)
        .await?;
    let Some(hash) = hash else { return Ok(Stage::WrongPassword) };
    if !hash.trim().is_empty() {
        let supplied = current_password.unwrap_or_default().to_string();
        if supplied.is_empty() || !libs::argon2::verify(&supplied, hash.clone()) {
            return Ok(Stage::WrongPassword);
        }
    }

    // Reject if another account already owns this email.
    let taken: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE LOWER(email) = $1 AND id <> $2")
            .bind(new_email)
            .bind(user_id)
            .fetch_one(&mut *conn)
            .await?;
    if taken > 0 {
        return Ok(Stage::Taken);
    }

    // Only while the password is still the one just checked.
    let done = sqlx::query(
        "UPDATE users SET pending_email = $1, pending_email_otp = $2 \
         WHERE id = $3 AND password_hash = $4",
    )
    .bind(new_email)
    .bind(otp)
    .bind(user_id)
    .bind(&hash)
    .execute(&mut *conn)
    .await?;
    Ok(if done.rows_affected() == 1 { Stage::Staged } else { Stage::WrongPassword })
}

#[cfg(test)]
mod tests {
    //! These run against a Postgres that has the api_switch migrations applied,
    //! named by SILOCAT_TEST_DATABASE_URL, and skip when it is unset. Each test
    //! works inside one transaction and never commits it.

    use super::*;
    use crate::libs::{apikey, rng};
    use sqlx::{Connection, PgConnection};

    async fn connect() -> Option<PgConnection> {
        let url = std::env::var("SILOCAT_TEST_DATABASE_URL").ok()?;
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        Some(PgConnection::connect(&url).await.expect("connect to SILOCAT_TEST_DATABASE_URL"))
    }

    /// A verified account whose password hash is `password_hash` ('' for a
    /// Google sign-up that never set one).
    async fn seed(conn: &mut PgConnection, password_hash: &str) -> String {
        let id = rng::uuid();
        let key = apikey::mint().expect("test key");
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
             api_key_migrated, account_type, email_verified) \
             VALUES ($1, $2, $3, $4, '', $5, $6, TRUE, 'personal', true)",
        )
        .bind(&id)
        .bind(format!("owner{}", rng::string(10)).to_lowercase())
        .bind(format!("{}@example.com", id))
        .bind(password_hash)
        .bind(&key.blind_index)
        .bind(&key.encrypted)
        .execute(&mut *conn)
        .await
        .expect("seed user");
        id
    }

    async fn pending(conn: &mut PgConnection, id: &str) -> (Option<String>, Option<String>) {
        sqlx::query_as("SELECT pending_email, pending_email_otp FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&mut *conn)
            .await
            .expect("read pending")
    }

    #[tokio::test]
    async fn a_key_alone_cannot_stage_an_email_change() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let hash = libs::argon2::hash("owner-password-1".to_string()).unwrap();
        let id = seed(&mut tx, &hash).await;

        for supplied in [None, Some(""), Some("not-the-password")] {
            let out = stage(&mut tx, &id, supplied, "attacker@evil.example", "111111").await.unwrap();
            assert_eq!(out, Stage::WrongPassword, "staged with {:?}", supplied);
            assert_eq!(pending(&mut tx, &id).await, (None, None));
        }

        let out = stage(&mut tx, &id, Some("owner-password-1"), "new@example.net", "222222").await.unwrap();
        assert_eq!(out, Stage::Staged);
        assert_eq!(
            pending(&mut tx, &id).await,
            (Some("new@example.net".to_string()), Some("222222".to_string()))
        );
    }

    #[tokio::test]
    async fn an_account_without_a_password_stages_without_one() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let id = seed(&mut tx, "").await;

        let out = stage(&mut tx, &id, None, "new@example.net", "333333").await.unwrap();
        assert_eq!(out, Stage::Staged);
        assert_eq!(pending(&mut tx, &id).await.0.as_deref(), Some("new@example.net"));
    }

    #[tokio::test]
    async fn an_address_another_account_owns_is_refused() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let id = seed(&mut tx, "").await;
        let other = seed(&mut tx, "").await;
        let other_email = format!("{}@example.com", other);

        let out = stage(&mut tx, &id, None, &other_email, "444444").await.unwrap();
        assert_eq!(out, Stage::Taken);
        assert_eq!(pending(&mut tx, &id).await, (None, None));
    }
}
