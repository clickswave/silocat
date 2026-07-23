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
            let retry_after = 60 - diff.num_seconds();
            return respond(
                429,
                "Too Many Requests",
                vec![format!("Please wait {} seconds before requesting another code", retry_after)],
                json!({ "retry_after": retry_after }),
            );
        }
    }

    let otp = libs::rng::number(6);

    if let Err(e) = sqlx::query("UPDATE users SET otp = $1, otp_last_sent_at = NOW(), otp_expires_at = NOW() + INTERVAL '10 minutes', otp_attempts = 0 WHERE id = $2")
        .bind(&otp)
        .bind(&user_id)
        .execute(&state.pg_pool)
        .await
    {
        println!("[FORGOT_PW] otp store failed: {:?}", e);
        return respond(500, "Something went wrong", vec![], json!({}));
    }

    if let Err(e) =
        libs::email::send_password_reset_email(&state.smtp_config, &username, &user_email, &otp)
            .await
    {
        println!("[FORGOT_PW] email send failed: {}", e);
        // Don't reveal the failure specifics; the code is stored, they can retry.
        return respond(500, "Could not send the reset email", vec![], json!({}));
    }

    generic_ok()
}
