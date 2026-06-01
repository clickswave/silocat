use axum::{extract::State, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{libs, models};
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct ChangePasswordInput {
    pub user_id: String,
    pub current_password: String,
    pub new_password: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<ChangePasswordInput>,
) -> impl IntoResponse {
    // Input validation for new password
    if let Err(errors) = libs::input_validators::password(&payload.new_password) {
        return respond(
            400,
            "Invalid new password",
            errors,
            json!({}),
        );
    }

    // Fetch user
    let find_user_query = sqlx::query_as::<_, models::User>(
        "SELECT * FROM users WHERE id = $1"
    )
    .bind(payload.user_id)
    .fetch_one(&axum_state.pg_pool)
    .await;

    let user = match find_user_query {
        Ok(user) => user,
        Err(_) => {
            return respond(
                404,
                "User not found",
                vec!["Could not find user account".to_string()],
                json!({}),
            );
        }
    };

    // Verify current password
    // IMPORTANT: In a real app, verify `current_password` against `user.password_hash`
    // using Argon2. We rely on libs::argon2::verify here.
    let is_valid = libs::argon2::verify(&payload.current_password, user.password_hash);

    if !is_valid {
        return respond(
            401,
            "Authentication failed",
            vec!["Current password is incorrect".to_string()],
            json!({}),
        );
    }

    // Hash new password
    let new_hash = match libs::argon2::hash(payload.new_password) {
        Ok(hash) => hash,
        Err(_) => {
            return respond(
                500,
                "Internal Error",
                vec!["Failed to process new password".to_string()],
                json!({}),
            );
        }
    };

    // Update user record
    let update_query = sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        new_hash,
        user.id
    )
    .execute(&axum_state.pg_pool)
    .await;

    match update_query {
        Ok(_) => {
            respond(
                200,
                "Password updated",
                vec![],
                json!({}),
            )
        }
        Err(_) => {
            respond(
                500,
                "Update failed",
                vec!["Database error while updating password".to_string()],
                json!({}),
            )
        }
    }
}
