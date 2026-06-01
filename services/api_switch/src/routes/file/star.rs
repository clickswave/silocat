use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};


use crate::routes::respond;
use serde_json::json;

#[derive(Deserialize)]
pub struct StarFilePayload {
    pub file_id: String,
    pub user_id: String,
    pub starred: bool,
}

#[derive(Deserialize)]
pub struct StarFolderPayload {
    pub folder_id: String,
    pub user_id: String,
    pub starred: bool,
}

pub async fn file(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<StarFilePayload>,
) -> impl IntoResponse {
    // Ensure IDs are valid UUIDs if strictly required, but let's try raw query first.
    // If sqlx fails to cast String to UUID, we will see it in the error log.

    let result = sqlx::query!(
        "UPDATE files SET starred = $1 WHERE id = $2 AND user_id = $3 RETURNING id",
        payload.starred,
        payload.file_id, // SQLx usually handles String -> UUID if formatted correctly
        payload.user_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match result {
        Ok(Some(_)) => {
            respond(200, "File star status updated", vec![], json!({}))
        },
        Ok(None) => {
            respond(404, "File not found or access denied", vec!["File not found".to_string()], json!({}))
        },
        Err(e) => {
            respond(500, "Database error", vec![e.to_string()], json!({}))
        },
    }
}

pub async fn folder(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<StarFolderPayload>,
) -> impl IntoResponse {
    let result = sqlx::query!(
        "UPDATE folders SET starred = $1 WHERE id = $2 AND user_id = $3 RETURNING id",
        payload.starred,
        payload.folder_id,
        payload.user_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match result {
        Ok(Some(_)) => respond(200, "Folder star status updated", vec![], json!({})),
        Ok(None) => respond(404, "Folder not found or access denied", vec!["Folder not found".to_string()], json!({})),
        Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
    }
}
