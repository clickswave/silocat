use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;

use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use serde_json::json;

#[derive(Deserialize)]
pub struct StarFilePayload {
    pub file_id: String,
    pub starred: bool,
}

#[derive(Deserialize)]
pub struct StarFolderPayload {
    pub folder_id: String,
    pub starred: bool,
}

pub async fn file(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<StarFilePayload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    let user_id = match caller.as_ref().and_then(|c| c.user_id.clone()) {
        Some(uid) => uid,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    let result = sqlx::query!(
        "UPDATE files SET starred = $1 WHERE id = $2 AND user_id = $3 RETURNING id",
        payload.starred,
        payload.file_id,
        user_id
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
        Err(_e) => {
            respond(500, "Database error", vec![], json!({}))
        },
    }
}

pub async fn folder(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<StarFolderPayload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    let user_id = match caller.as_ref().and_then(|c| c.user_id.clone()) {
        Some(uid) => uid,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    let result = sqlx::query!(
        "UPDATE folders SET starred = $1 WHERE id = $2 AND user_id = $3 RETURNING id",
        payload.starred,
        payload.folder_id,
        user_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match result {
        Ok(Some(_)) => respond(200, "Folder star status updated", vec![], json!({})),
        Ok(None) => respond(404, "Folder not found or access denied", vec!["Folder not found".to_string()], json!({})),
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}
