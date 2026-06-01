use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub folder_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Fetch Folder Metadata
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
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
    };

    // 2. Fetch Child Files
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
        Err(e) => return respond(500, "Failed to fetch files", vec![e.to_string()], json!({})),
    };

    // 3. Fetch Child Folders
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
        Err(e) => return respond(500, "Failed to fetch subfolders", vec![e.to_string()], json!({})),
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
