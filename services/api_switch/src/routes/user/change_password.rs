use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, models};
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct ChangePasswordInput {
    // Required only when the account already has a password. Google accounts that
    // have never set one can leave this empty ("set password" flow).
    pub current_password: Option<String>,
    pub new_password: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(token_user): Extension<models::UserTokenData>,
    Json(payload): Json<ChangePasswordInput>,
) -> impl IntoResponse {
    // Validate the new password.
    if let Err(errors) = libs::input_validators::password(&payload.new_password) {
        return respond(400, "Invalid new password", errors, json!({}));
    }

    // Always operate on the authenticated user from the token (not a client id).
    let user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(&token_user.id)
        .fetch_one(&axum_state.pg_pool)
        .await
    {
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

    let has_password = !user.password_hash.trim().is_empty();

    // If a password already exists, the current one must be supplied and correct.
    // If none exists (Google sign-up), this is a first-time "set password".
    if has_password {
        let current = payload.current_password.clone().unwrap_or_default();
        if current.is_empty() || !libs::argon2::verify(&current, user.password_hash.clone()) {
            return respond(
                401,
                "Authentication failed",
                vec!["Current password is incorrect".to_string()],
                json!({}),
            );
        }
    }

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

    match sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&new_hash)
        .bind(&user.id)
        .execute(&axum_state.pg_pool)
        .await
    {
        Ok(_) => respond(
            200,
            if has_password { "Password updated" } else { "Password set" },
            vec![],
            json!({ "password_set": true }),
        ),
        Err(_) => respond(
            500,
            "Update failed",
            vec!["Database error while updating password".to_string()],
            json!({}),
        ),
    }
}
