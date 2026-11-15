use crate::middlewares::validate_token::PresentedKey;
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
    Extension(key): Extension<PresentedKey>,
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

    // Everything below runs while the account is locked with the key this
    // request came in with, since the response hands that key back. A request
    // let in just before the key was replaced would otherwise wait for the
    // change and then be given the new key.
    let mut tx = match axum_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_) => return respond(500, "Verification failed", vec!["Internal server error".to_string()], json!({})),
    };
    match crate::libs::key_guard::still_current(&mut tx, &user.id, &key.0).await {
        Ok(true) => {}
        Ok(false) => {
            return respond(401, "Unauthorized", vec!["Invalid or missing API key".to_string()], json!({}))
        }
        Err(_) => return respond(500, "Verification failed", vec!["Internal server error".to_string()], json!({})),
    }

    // Verify + consume the code atomically (expiry + attempt limit enforced).
    match crate::libs::otp::consume(&mut *tx, &user.id, &payload.otp).await {
        crate::libs::otp::Outcome::Valid => {}
        crate::libs::otp::Outcome::Invalid => {
            // Commit, so the failed attempt still counts against the limit.
            let _ = tx.commit().await;
            return respond(
                400,
                "Invalid OTP",
                vec!["The OTP you entered is incorrect or has expired".to_string()],
                json!({}),
            );
        }
    }

    // OTP matches, verify user
    if let Err(_) = sqlx::query!(
        "UPDATE users SET email_verified = true, otp = '' WHERE id = $1",
        user.id
    )
    .execute(&mut *tx)
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
        .fetch_one(&mut *tx)
        .await
    {
        Ok(u) => u,
        Err(_e) => return respond(500, "Failed to fetch updated profile", vec![], json!({})),
    };
    if let Err(_) = tx.commit().await {
        return respond(500, "Verification failed", vec!["Internal server error".to_string()], json!({}));
    }

    let token_data = crate::models::token_data(updated_user, user.subscription);

    respond(
        200,
        "Email verified successfully",
        vec![],
        json!(token_data),
    )
}
