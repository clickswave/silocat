use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, models::UserTokenData, routes::respond};

#[derive(Deserialize)]
pub struct Input {
    pub email: String,
}

// Stage a new email + OTP without touching the live (verified) account email.
// Also used as the "resend" action: calling again regenerates + resends the code.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Json(payload): Json<Input>,
) -> impl IntoResponse {
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

    // Reject if another account already owns this email.
    let taken = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE LOWER(email) = $1 AND id <> $2",
    )
    .bind(&new_email)
    .bind(user.id.clone())
    .fetch_one(&state.pg_pool)
    .await;
    match taken {
        Ok(c) if c > 0 => {
            return respond(
                409,
                "Email already in use",
                vec!["That email is already registered to another account.".to_string()],
                json!({}),
            )
        }
        Ok(_) => {}
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
    }

    let otp = libs::rng::number(6);
    if let Err(e) = sqlx::query(
        "UPDATE users SET pending_email = $1, pending_email_otp = $2 WHERE id = $3",
    )
    .bind(&new_email)
    .bind(&otp)
    .bind(user.id.clone())
    .execute(&state.pg_pool)
    .await
    {
        return respond(500, "Failed to start email change", vec![e.to_string()], json!({}));
    }

    if let Err(e) =
        libs::email::send_verification_email(&state.smtp_config, &user.username, &new_email, &otp).await
    {
        println!("Failed to send email-change code to {}: {}", new_email, e);
        return respond(500, "Could not send verification email", vec![], json!({}));
    }

    respond(200, "Verification code sent", vec![], json!({ "pending_email": new_email }))
}
