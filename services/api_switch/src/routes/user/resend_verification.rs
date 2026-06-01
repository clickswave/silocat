use crate::libs;
use crate::models::UserTokenData;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde_json::json;

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
) -> impl IntoResponse {
    
    if user.email_verified {
        return respond(
            400,
            "Already verified",
            vec!["Your email is already verified".to_string()],
            json!({}),
        );
    }

    // Check last sent time
    let db_user = match sqlx::query!("SELECT otp_last_sent_at FROM users WHERE id = $1", user.id)
        .fetch_optional(&axum_state.pg_pool)
        .await {
        Ok(Some(u)) => u,
        _ => return respond(500, "User not found", vec![], json!({})),
    };

    if let Some(last_sent) = db_user.otp_last_sent_at {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(last_sent);
        if diff.num_seconds() < 60 {
             let retry_after = 60 - diff.num_seconds();
             return respond(
                429,
                "Too Many Requests",
                vec![format!("Please wait {} seconds before resending", retry_after)],
                json!({ "retry_after": retry_after }),
            );
        }
    }

    let otp = libs::rng::number(6);

    // Update user with new OTP and timestamp
    if let Err(e) = sqlx::query!(
        "UPDATE users SET otp = $1, otp_last_sent_at = NOW() WHERE id = $2",
        otp,
        user.id
    )
    .execute(&axum_state.pg_pool)
    .await {
         return respond(
            500,
            "Failed to update OTP",
            vec![e.to_string()],
            json!({}),
        );
    }

    // send verification email
    if let Err(e) = libs::email::send_verification_email(
        &axum_state.smtp_config,
        &user.username,
        &user.email,
        &otp
    ).await {
         return respond(
            500,
            "Failed to send email",
            vec![e.to_string()],
            json!({}),
        );
    }

    respond(
        200,
        "Verification code sent",
        vec![],
        json!({}),
    )
}
