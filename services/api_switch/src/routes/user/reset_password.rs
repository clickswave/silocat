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
    State(state): State<crate::AppState>,
    Json(payload): Json<ResetPasswordInput>,
) -> impl IntoResponse {
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

    // An empty stored OTP means there's no active reset request; never match it.
    if user.otp.trim().is_empty() || user.otp != otp {
        return respond(
            400,
            "Invalid or expired code",
            vec!["The code is incorrect or has expired.".to_string()],
            json!({}),
        );
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
        sqlx::query_as::<_, models::Subscription>("SELECT * FROM subscriptions WHERE id = $1")
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
        Err(e) => return respond(500, "Failed to load profile", vec![e.to_string()], json!({})),
    };

    let token_data = models::token_data(updated, subscription);
    respond(200, "Password reset successful", vec![], json!({ "user": token_data }))
}
