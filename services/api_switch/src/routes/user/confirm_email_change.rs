use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::validate_token::PresentedKey;
use crate::{models, models::UserTokenData, routes::respond};

#[derive(Deserialize)]
pub struct Input {
    pub otp: String,
}

// Confirm a staged email change: verify the OTP, then swap the account email
// to the pending address (already verified by virtue of the code) and clear it.
//
// All of it runs in one transaction that first locks the account while it
// still has the key this request came in with. A request let in just before
// the owner of the address claimed the account would otherwise read the
// registrant's pending address, wait for the claim, then move the account to
// that address, mark it verified and return the key the claim minted.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Extension(key): Extension<PresentedKey>,
    Json(payload): Json<Input>,
) -> impl IntoResponse {
    let mut tx = match state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };
    match crate::libs::key_guard::still_current(&mut tx, &user.id, &key.0).await {
        Ok(true) => {}
        Ok(false) => {
            return respond(401, "Unauthorized", vec!["Invalid or missing API key".to_string()], json!({}))
        }
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    }

    let row = sqlx::query_as::<_, (Option<String>, Option<String>)>(
        "SELECT pending_email, pending_email_otp FROM users WHERE id = $1",
    )
    .bind(user.id.clone())
    .fetch_optional(&mut *tx)
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

    // Brute-force guard. The pending OTP is a 6-digit code (~10^6 space) with no
    // built-in TTL, so cap confirmation attempts: 5 per 10 minutes per account
    // makes searching the space infeasible. Compare in constant time so a timing
    // side channel can't leak the code either.
    if !state.rate_limiter.check(
        &format!("emailchange:confirm:{}", user.id), 5, std::time::Duration::from_secs(600))
    {
        return respond(429, "Too Many Requests",
            vec!["Too many attempts. Please wait and try again.".to_string()], json!({}));
    }

    if !crate::middlewares::ct_eq(payload.otp.trim().as_bytes(), pending_otp.as_bytes()) {
        return respond(400, "Invalid OTP", vec!["The code you entered is incorrect.".to_string()], json!({}));
    }

    // Re-check uniqueness in case the address was taken since the request.
    let taken = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM users WHERE LOWER(email) = $1 AND id <> $2",
    )
    .bind(pending_email.to_lowercase())
    .bind(user.id.clone())
    .fetch_one(&mut *tx)
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
         pending_email_otp = NULL, reset_code = '' WHERE id = $2",
    )
    .bind(&pending_email)
    .bind(user.id.clone())
    .execute(&mut *tx)
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
        .fetch_one(&mut *tx)
        .await
    {
        Ok(u) => u,
        Err(_e) => return respond(500, "Failed to fetch updated profile", vec![], json!({})),
    };
    if let Err(_e) = tx.commit().await {
        return respond(500, "Failed to update email", vec![], json!({}));
    }

    let token_data = models::token_data(updated_user, user.subscription.clone());
    respond(200, "Email updated successfully", vec![], json!(token_data))
}
