use axum::{extract::State, response::IntoResponse, Router, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use sqlx::types::JsonValue;
use crate::routes::respond;


// We need to define a local struct because AnonUser is not fully defined in models.rs (it was missed in previous context or only sql exists)
// Looking at models.rs content from previous turn... it wasn't there!
// So I will define a local struct here matching the DB schema.

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct AnonymousUser {
    pub api_key: String,
    pub ip_address: String,
    pub user_agent: Option<String>,
    pub geo_location: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub bandwidth_usage_bytes: i64,
    pub last_reset_stats: DateTime<Utc>,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_anon_users))
}

async fn list_anon_users(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let users = sqlx::query_as!(
        AnonymousUser,
        "SELECT * FROM anonymous_users ORDER BY last_seen DESC LIMIT 100"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match users {
        Ok(users) => respond(
            200,
            "Anonymous users fetched successfully",
            vec![],
            json!({ "users": users }),
        ),
        Err(_e) => respond(
            500,
            "Failed to fetch anonymous users",
            vec![],
            json!({}),
        ),
    }
}
