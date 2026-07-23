use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Try to find as File
    let file_query = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE id = $1",
        payload.id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file_query {
        Ok(Some(file)) => {
            let allowed = file.public_access
                || caller
                    .as_ref()
                    .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key));
            if !allowed {
                return respond(404, "Resource not found", vec![], json!({}));
            }
            return respond(
                200,
                "Resource found (File)",
                vec![],
                json!({
                    "resource_type": "file",
                    "file": file
                }),
            );
        }
        Ok(None) => {
            // Not a file, proceed to check folders
        }
        Err(_e) => {
            return respond(500, "Database error checking file", vec![], json!({}));
        }
    }

    // 2. Try to find as Folder
    // We need to fetch folder metadata + contents to be useful
    // But let's just return the folder metadata first, knowing the frontend might need another call 
    // OR we can reuse the logic from fetch_folder.rs to return everything.
    // For now, let's just return the folder object and let the frontend decide if it needs children.
    // Actually, fetch_folder.rs returns { folder, files, folders }. The frontend likely expects this structure.
    // So let's duplicate that logic or reuse it? Duplication is safer for now to avoid borrowing issues or refactoring.
    
    let folder_query = sqlx::query_as!(
        models::Folder,
        "SELECT * FROM folders WHERE id = $1",
        payload.id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder_query {
        Ok(Some(folder)) => {
            // Owner-only: public folders are served via the share-token path.
            let allowed = caller
                .as_ref()
                .map_or(false, |c| c.owns(&folder.user_id, &folder.owner_api_key));
            if !allowed {
                return respond(404, "Resource not found", vec!["ID does not match any file or folder".to_string()], json!({}));
            }
            // Fetch children too? Yes, usually needed.
            let files_query = sqlx::query_as!(
                models::File,
                "SELECT * FROM files WHERE folder_id = $1 AND deleted = false",
                folder.id
            )
            .fetch_all(&axum_state.pg_pool)
            .await;

            let files = match files_query {
                Ok(f) => f,
                Err(_) => vec![], // Ignore error here for robust fetching? Or fail?
            };

            let subfolders_query = sqlx::query_as!(
                models::Folder,
                "SELECT * FROM folders WHERE parent_id = $1",
                folder.id
            )
            .fetch_all(&axum_state.pg_pool)
            .await;

             let subfolders = match subfolders_query {
                Ok(f) => f,
                Err(_) => vec![],
            };

            return respond(
                200,
                "Resource found (Folder)",
                vec![],
                json!({
                    "resource_type": "folder",
                    "folder": folder,
                    "files": files,
                    "folders": subfolders
                }),
            );
        }
        Ok(None) => {
             // Not a file, not a folder
             return respond(404, "Resource not found", vec!["ID does not match any file or folder".to_string()], json!({}));
        }
        Err(_e) => {
            return respond(500, "Database error checking folder", vec![], json!({}));
        }
    }
}
