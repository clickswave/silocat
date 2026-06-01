use axum::{extract::State, response::IntoResponse, Router, routing::get};
use serde_json::json;
use crate::{models};
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", axum::routing::delete(delete_user))
}

async fn list_users(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let users = sqlx::query_as!(
        models::User,
        "SELECT * FROM users ORDER BY created_on DESC LIMIT 100" // Simple limit for now
    )
    .fetch_all(&state.pg_pool)
    .await;

    match users {
        Ok(users) => respond(
            200,
            "Users fetched successfully",
            vec![],
            json!({ "users": users }),
        ),
        Err(e) => respond(
            500,
            "Failed to fetch users",
            vec![e.to_string()],
            json!({}),
        ),
    }
}

async fn delete_user(
    State(state): State<crate::AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query!("DELETE FROM users WHERE id = $1", id)
        .execute(&state.pg_pool)
        .await;

    match result {
        Ok(_) => respond(
            200,
            "User deleted successfully",
            vec![],
            json!({}),
        ),
        Err(e) => respond(
            500,
            "Failed to delete user",
            vec![e.to_string()],
            json!({}),
        ),
    }
}
