use axum::{extract::State, Json, response::IntoResponse, Router, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, types::chrono::{DateTime, Utc}};
use crate::routes::respond;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct File {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub created_on: DateTime<Utc>,
    pub downloads: i64,
    pub public_access: bool,
}

pub async fn list_files(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let files = sqlx::query_as!(
        File,
        "SELECT id, user_id, name, mime, size, created_on, downloads, public_access FROM files ORDER BY created_on DESC LIMIT 100"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match files {
        Ok(files) => respond(
            200,
            "Files retrieved successfully",
            vec![],
            json!({ "files": files }),
        ),
        Err(e) => respond(
            500,
            "Failed to retrieve files",
            vec![e.to_string()],
            json!({}),
        ),
    }
}

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", get(list_files))
}
