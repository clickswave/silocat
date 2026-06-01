use crate::models::UserTokenData;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct VerifyEmailInput {
    pub otp: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Json(payload): Json<VerifyEmailInput>,
) -> impl IntoResponse {
    // Check if the user is already verified
    if user.email_verified {
        return respond(
            400,
            "Already verified",
            vec!["Your email is already verified".to_string()],
            json!({}),
        );
    }

    // Since we don't have the OTP in UserTokenData (security), we need to fetch it or check it against DB.
    // Wait, UserTokenData doesn't have OTP. We need to query the DB for the stored OTP.
    
    let db_user = match sqlx::query!(
        "SELECT otp FROM users WHERE id = $1",
        user.id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await {
        Ok(Some(u)) => u,
        _ => return respond(500, "User not found", vec![], json!({})),
    };

    if db_user.otp != payload.otp {
         return respond(
            400,
            "Invalid OTP",
            vec!["The OTP you entered is incorrect".to_string()],
            json!({}),
        );
    }

    // OTP matches, verify user
    // OTP matches, verify user
    if let Err(_) = sqlx::query!(
        "UPDATE users SET email_verified = true, otp = '' WHERE id = $1",
        user.id
    )
    .execute(&axum_state.pg_pool)
    .await {
         return respond(
            500,
            "Verification failed",
            vec!["Internal server error".to_string()],
            json!({}),
        );
    }

    // Fetch updated user to return fresh token data
     let updated_user = match sqlx::query_as::<_, crate::models::User>("SELECT * FROM users WHERE id = $1")
        .bind(user.id)
        .fetch_one(&axum_state.pg_pool)
        .await
    {
        Ok(u) => u,
        Err(e) => return respond(500, "Failed to fetch updated profile", vec![e.to_string()], json!({})),
    };

    let token_data = crate::models::token_data(updated_user, user.subscription);

    respond(
        200,
        "Email verified successfully",
        vec![],
        json!(token_data),
    )
}
