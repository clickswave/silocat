use crate::libs::sharing;
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
            // A non-owner reads by id only via public_access, and only when no
            // active share protection applies (password / one-time / expiry /
            // delete); protected shares must use the token path.
            let share_protected = sharing::share_protected(
                file.share_password_hash.as_deref(),
                file.share_type.as_deref(),
                file.share_expires_at,
                file.deleted,
            );
            let allowed = caller
                .as_ref()
                .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key))
                || (file.public_access && !share_protected);
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
    let folder_query = sqlx::query_as!(
        models::Folder,
        "SELECT * FROM folders WHERE id = $1",
        payload.id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder_query {
        Ok(Some(folder)) => {
            // Same rule as a file, and for the same reason. A folder had no
            // `public_access` column at all until 0045, so this branch was
            // owner-only and every anonymous folder drop was a link that 404'd
            // for the person it was sent to.
            //
            // Read from the whole chain rather than this row: a subfolder of a
            // published folder is part of what was published, and a protection
            // anywhere above has to cover everything below it.
            let access = sharing::folder_access(&axum_state.pg_pool, &folder.id).await;
            let allowed = caller
                .as_ref()
                .map_or(false, |c| c.owns(&folder.user_id, &folder.owner_api_key))
                || access.grants_public_read();
            if !allowed {
                return respond(404, "Resource not found", vec!["ID does not match any file or folder".to_string()], json!({}));
            }

            // The whole subtree, not just the direct children: a folder upload
            // with any structure to it put its files in child folders, and
            // listing one level meant the recipient saw a fraction of the drop.
            let subtree = match sharing::folder_subtree(&axum_state.pg_pool, &folder.id).await {
                Ok(t) => t,
                Err(_e) => return respond(500, "Database error reading folder", vec![], json!({})),
            };
            let files = match sharing::subtree_files(&axum_state.pg_pool, &subtree).await {
                Ok(f) => f,
                Err(_e) => return respond(500, "Database error reading folder", vec![], json!({})),
            };

            let files_json: Vec<serde_json::Value> = files
                .iter()
                .map(|(file, path)| sharing::file_with_path(file, path))
                .collect();
            // Every folder below the root, so a client can show the structure
            // even where a directory holds nothing but more directories.
            let folders_json: Vec<serde_json::Value> = subtree
                .iter()
                .filter(|f| f.id != folder.id)
                .map(|f| json!({ "id": f.id, "path": f.path }))
                .collect();

            return respond(
                200,
                "Resource found (Folder)",
                vec![],
                json!({
                    "resource_type": "folder",
                    "folder": folder,
                    "files": files_json,
                    "folders": folders_json
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
