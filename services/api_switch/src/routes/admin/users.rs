use axum::{extract::{State, Path}, response::IntoResponse, Router, routing::{get, post}, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{models};
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", axum::routing::delete(delete_user))
        .route("/{id}/ban", post(ban_user))
        .route("/{id}/unban", post(unban_user))
        .route("/{id}/restrict", post(restrict_user))
}

#[derive(Deserialize)]
struct BanInput {
    reason: Option<String>,
    // None / 0 / negative => permanent.
    days: Option<i64>,
}

async fn ban_user(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
    Json(body): Json<BanInput>,
) -> impl IntoResponse {
    let permanent = body.days.map(|d| d <= 0).unwrap_or(true);
    let res = if permanent {
        sqlx::query(
            "UPDATE users SET is_banned = TRUE, banned_until = NULL, ban_reason = $1 WHERE id = $2",
        )
        .bind(&body.reason)
        .bind(&id)
        .execute(&state.pg_pool)
        .await
    } else {
        let until = chrono::Utc::now() + chrono::Duration::days(body.days.unwrap());
        sqlx::query(
            "UPDATE users SET is_banned = TRUE, banned_until = $1, ban_reason = $2 WHERE id = $3",
        )
        .bind(until)
        .bind(&body.reason)
        .bind(&id)
        .execute(&state.pg_pool)
        .await
    };
    match res {
        Ok(r) if r.rows_affected() == 0 => respond(404, "User not found", vec![], json!({})),
        Ok(_) => respond(200, "User banned", vec![], json!({})),
        Err(e) => respond(500, "Failed to ban user", vec![e.to_string()], json!({})),
    }
}

async fn unban_user(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let res = sqlx::query(
        "UPDATE users SET is_banned = FALSE, banned_until = NULL, ban_reason = NULL WHERE id = $1",
    )
    .bind(&id)
    .execute(&state.pg_pool)
    .await;
    match res {
        Ok(_) => respond(200, "User unbanned", vec![], json!({})),
        Err(e) => respond(500, "Failed to unban user", vec![e.to_string()], json!({})),
    }
}

#[derive(Deserialize)]
struct RestrictInput {
    restricted: bool,
}

async fn restrict_user(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
    Json(body): Json<RestrictInput>,
) -> impl IntoResponse {
    let res = sqlx::query("UPDATE users SET is_restricted = $1 WHERE id = $2")
        .bind(body.restricted)
        .bind(&id)
        .execute(&state.pg_pool)
        .await;
    match res {
        Ok(_) => respond(
            200,
            if body.restricted { "User restricted" } else { "User unrestricted" },
            vec![],
            json!({}),
        ),
        Err(e) => respond(500, "Failed to update restriction", vec![e.to_string()], json!({})),
    }
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
