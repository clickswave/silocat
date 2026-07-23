use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{models, models::UserTokenData, routes::respond};

#[derive(Deserialize)]
pub struct Input {
    pub otp: String,
}

// Confirm a staged email change: verify the OTP, then swap the account email
// to the pending address (already verified by virtue of the code) and clear it.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Json(payload): Json<Input>,
) -> impl IntoResponse {
    let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT pending_email, pending_email_otp FROM users WHERE id = $1",
    )
    .bind(user.id.clone())
    .fetch_optional(&state.pg_pool)
    .await;

    let (pending_email, pending_otp) = match row {
        Ok(Some((Some(e), Some(o)))) if !e.is_empty() => (e, o),
        Ok(_) => {
            return respond(
                400,
                "No pending email change",
                vec!["Request an email change first.".to_string()],
                json!({}),
            )
        }
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    if payload.otp.trim() != pending_otp {
        return respond(400, "Invalid OTP", vec!["The code you entered is incorrect.".to_string()], json!({}));
    }

    // Re-check uniqueness in case the address was taken since the request.
    let taken = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE LOWER(email) = $1 AND id <> $2",
    )
    .bind(pending_email.to_lowercase())
    .bind(user.id.clone())
    .fetch_one(&state.pg_pool)
    .await;
    if let Ok(c) = taken {
        if c > 0 {
            return respond(
                409,
                "Email already in use",
                vec!["That email is already registered to another account.".to_string()],
                json!({}),
            );
        }
    }

    if let Err(e) = sqlx::query(
        "UPDATE users SET email = $1, email_verified = true, pending_email = NULL, \
         pending_email_otp = NULL WHERE id = $2",
    )
    .bind(&pending_email)
    .bind(user.id.clone())
    .execute(&state.pg_pool)
    .await
    {
        if e.as_database_error().map(|db| db.is_unique_violation()).unwrap_or(false) {
            return respond(
                409,
                "Email already in use",
                vec!["That email is already registered to another account.".to_string()],
                json!({}),
            );
        }
        return respond(500, "Failed to update email", vec![], json!({}));
    }

    let updated_user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(user.id.clone())
        .fetch_one(&state.pg_pool)
        .await
    {
        Ok(u) => u,
        Err(_e) => return respond(500, "Failed to fetch updated profile", vec![], json!({})),
    };

    let token_data = models::token_data(updated_user, user.subscription.clone());
    respond(200, "Email updated successfully", vec![], json!(token_data))
}
