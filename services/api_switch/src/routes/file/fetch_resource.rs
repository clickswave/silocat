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
            // A non-owner reads by id only an anonymous drop, and only when no
            // active share protection applies (password / one-time / expiry /
            // delete). A drive file is shared through its token and nothing else.
            let share_protected = sharing::share_protected(
                file.share_password_hash.as_deref(),
                file.share_type.as_deref(),
                file.share_expires_at,
                file.deleted,
            );
            let owns = caller
                .as_ref()
                .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key));
            // The folders above have a say too, as in fetch-chunks: a trashed
            // or protected folder covers every file under it.
            let folder_access = match file.folder_id.as_deref() {
                Some(folder_id) if !owns => sharing::folder_access(&axum_state.pg_pool, folder_id).await,
                _ => sharing::FolderAccess::none(),
            };
            let allowed = owns
                || sharing::file_readable_by_id(
                    file.user_id.as_deref(),
                    file.public_access,
                    share_protected,
                    &folder_access,
                );
            if !allowed {
                return respond(404, "Resource not found", vec![], json!({}));
            }
            return respond(
                200,
                "Resource found (File)",
                vec![],
                json!({
                    "resource_type": "file",
                    "file": file,
                    "sender_paid": crate::libs::plans::sender_paid(&axum_state.pg_pool, crate::libs::plans::SharedBy::Id(&payload.id)).await
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
            // anywhere above has to cover everything below it. A drive folder
            // never opens by id for a non-owner, only through its token.
            let access = sharing::folder_access(&axum_state.pg_pool, &folder.id).await;
            let allowed = caller
                .as_ref()
                .map_or(false, |c| c.owns(&folder.user_id, &folder.owner_api_key))
                || sharing::folder_readable_by_id(folder.user_id.as_deref(), &access);
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
                    "folders": folders_json,
                    "sender_paid": crate::libs::plans::sender_paid(&axum_state.pg_pool, crate::libs::plans::SharedBy::Id(&payload.id)).await
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

#[cfg(test)]
mod tests {
    //! These call the handlers against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset.

    use super::*;
    use crate::test_support::state;
    use axum::http::StatusCode;

    /// An anonymous folder drop is opened by its id. Once the folder is in the
    /// trash the folder id stops working, and so must the id of a file in it:
    /// both by-id file reads used to look at the file row alone.
    #[tokio::test]
    async fn a_file_in_a_trashed_drop_folder_is_not_readable_by_id() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let pool = &st.pg_pool;
        let drop_folder: String = sqlx::query_scalar(
            "INSERT INTO folders (name, user_id, public_access) VALUES ('drop', NULL, true) RETURNING id",
        )
        .fetch_one(pool)
        .await
        .expect("seed drop folder");
        let file_id: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, \
             sha256_checksum, blake3_checksum, public_access, folder_id) \
             VALUES (NULL, 'f.bin', 'application/octet-stream', 1, false, 1, 1, 's', '', true, $1) RETURNING id",
        )
        .bind(&drop_folder)
        .fetch_one(pool)
        .await
        .expect("seed drop file");

        async fn read(st: &crate::AppState, id: &str) -> (StatusCode, StatusCode) {
            let by_resource = handle(State(st.clone()), Extension(None), Json(PayloadBody { id: id.to_string() }))
                .await
                .into_response()
                .status();
            let by_file = super::super::fetch_files::handle(
                State(st.clone()),
                Extension(None),
                Json(super::super::fetch_files::PayloadBody { file_id: id.to_string() }),
            )
            .await
            .into_response()
            .status();
            (by_resource, by_file)
        }

        let before = read(&st, &file_id).await;
        sqlx::query("UPDATE folders SET deleted = true, deleted_on = NOW() WHERE id = $1")
            .bind(&drop_folder)
            .execute(pool)
            .await
            .expect("trash drop folder");
        let after = read(&st, &file_id).await;
        let _ = sqlx::query("DELETE FROM folders WHERE id = $1").bind(&drop_folder).execute(pool).await;

        assert_eq!(before, (StatusCode::OK, StatusCode::OK), "a live drop reads by id");
        assert_eq!(after, (StatusCode::NOT_FOUND, StatusCode::NOT_FOUND), "a file in a trashed drop folder");
    }
}
