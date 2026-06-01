use axum::{extract::State, Json, response::IntoResponse, Router, routing::get};
use serde::Serialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Serialize)]
pub struct DashboardStats {
    pub total_users: i64,
    pub total_files: i64,
    pub total_storage_bytes: i64,
    pub total_subscriptions: i64,
    pub active_anon_users: i64,
}

pub async fn get_stats(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let total_users = sqlx::query!("SELECT count(*) as count FROM users")
        .fetch_one(&state.pg_pool)
        .await
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);

    let total_files = sqlx::query!("SELECT count(*) as count FROM files")
        .fetch_one(&state.pg_pool)
        .await
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);

    let total_storage = sqlx::query!("SELECT coalesce(sum(size), 0)::BIGINT as size FROM files")
        .fetch_one(&state.pg_pool)
        .await
        .map(|r| r.size.unwrap_or(0))
        .unwrap_or(0);
        
    let total_subs = sqlx::query!("SELECT count(*) as count FROM subscriptions")
        .fetch_one(&state.pg_pool)
        .await
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);
        
    let active_anons = sqlx::query!("SELECT count(*) as count FROM anonymous_users WHERE last_seen > NOW() - INTERVAL '24 HOURS'")
        .fetch_one(&state.pg_pool)
        .await
        .map(|r| r.count.unwrap_or(0))
        .unwrap_or(0);

    let stats = DashboardStats {
        total_users,
        total_files,
        total_storage_bytes: total_storage,
        total_subscriptions: total_subs,
        active_anon_users: active_anons,
    };

    respond(
        200,
        "Stats retrieved successfully",
        vec![],
        json!({ "stats": stats }),
    )
}

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", get(get_stats))
}
