use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub folder_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Fetch Folder Metadata
    let folder_query = sqlx::query_as!(
        models::Folder,
        "SELECT * FROM folders WHERE id = $1",
        payload.folder_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    let folder = match folder_query {
        Ok(Some(f)) => f,
        Ok(None) => return respond(404, "Folder not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    // Owner-only: a folder's listing (and its children's metadata) is never
    // returned for another user's folder id. Ownership is derived from the
    // authenticated caller, never a client-supplied api key.
    let allowed = caller
        .as_ref()
        .map_or(false, |c| c.owns(&folder.user_id, &folder.owner_api_key));
    if !allowed {
        return respond(404, "Folder not found", vec![], json!({}));
    }

    // 2. Fetch Child Files
    let files_query = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE folder_id = $1 AND deleted = false",
        folder.id
    )
    .fetch_all(&axum_state.pg_pool)
    .await;

    let files = match files_query {
        Ok(files) => files,
        Err(_e) => return respond(500, "Failed to fetch files", vec![], json!({})),
    };

    // 3. Fetch Child Folders
    let subfolders_query = sqlx::query_as!(
        models::Folder,
        "SELECT * FROM folders WHERE parent_id = $1",
        folder.id
    )
    .fetch_all(&axum_state.pg_pool)
    .await;

    let subfolders = match subfolders_query {
        Ok(folders) => folders,
        Err(_e) => return respond(500, "Failed to fetch subfolders", vec![], json!({})),
    };

    respond(
        200,
        "Folder contents retrieved",
        vec![],
        json!({
            "folder": folder,
            "files": files,
            "folders": subfolders
        }),
    )
}
