use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, models, routes::respond};

#[derive(Deserialize, Debug)]
pub struct ResetPasswordInput {
    pub email: String,
    pub otp: String,
    pub new_password: String,
}

/// POST /user/reset-password  (public)
/// Verifies the emailed code and sets a new password. Proving control of the
/// inbox also verifies the email, so we mark it verified here. On success we
/// return fresh token data so the frontend can log the user straight in.
pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ResetPasswordInput>,
) -> impl IntoResponse {
    // Per-IP throttle on top of the per-code attempt limit, so the OTP space
    // can't be swept from one host.
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !state.rate_limiter.check(&format!("reset:{}", ip), 15, std::time::Duration::from_secs(600)) {
        return respond(
            429,
            "Too Many Requests",
            vec!["Too many attempts. Please try again later.".to_string()],
            json!({}),
        );
    }
    // Validate the new password against our rules.
    if let Err(errors) = libs::input_validators::password(&payload.new_password) {
        return respond(400, "Invalid new password", errors, json!({}));
    }

    let email = payload.email.trim().to_string();
    let otp = payload.otp.trim().to_string();
    if email.is_empty() || otp.is_empty() {
        return respond(400, "Email and code are required", vec![], json!({}));
    }

    let user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return respond(
                400,
                "Invalid or expired code",
                vec!["The code is incorrect or has expired.".to_string()],
                json!({}),
            )
        }
        Err(e) => {
            println!("[RESET_PW] lookup failed: {:?}", e);
            return respond(500, "Something went wrong", vec![], json!({}));
        }
    };

    // Verify + consume the code atomically: rejects an empty/expired code and
    // locks out after too many wrong guesses, so it can't be brute-forced.
    match libs::otp::consume(&state.pg_pool, &user.id, &otp).await {
        libs::otp::Outcome::Valid => {}
        libs::otp::Outcome::Invalid => {
            return respond(
                400,
                "Invalid or expired code",
                vec!["The code is incorrect or has expired.".to_string()],
                json!({}),
            );
        }
    }

    let new_hash = match libs::argon2::hash(payload.new_password) {
        Ok(h) => h,
        Err(_) => return respond(500, "Internal Error", vec!["Failed to process password".to_string()], json!({})),
    };

    if let Err(e) = sqlx::query(
        "UPDATE users SET password_hash = $1, otp = '', email_verified = true WHERE id = $2",
    )
    .bind(&new_hash)
    .bind(&user.id)
    .execute(&state.pg_pool)
    .await
    {
        println!("[RESET_PW] update failed: {:?}", e);
        return respond(500, "Could not reset password", vec![], json!({}));
    }

    // Return fresh token data (with the subscription) so the caller can sign in.
    let subscription = if let Some(sub_id) = &user.subscription_id {
        sqlx::query_as::<_, models::Subscription>("SELECT * FROM subscriptions WHERE id = $1 AND expires_on > NOW()")
            .bind(sub_id)
            .fetch_optional(&state.pg_pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let updated = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(&user.id)
        .fetch_one(&state.pg_pool)
        .await
    {
        Ok(u) => u,
        Err(_e) => return respond(500, "Failed to load profile", vec![], json!({})),
    };

    let token_data = models::token_data(updated, subscription);
    respond(200, "Password reset successful", vec![], json!({ "user": token_data }))
}
