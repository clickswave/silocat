use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct DeletePayload {
    pub folder_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<DeletePayload>,
) -> impl IntoResponse {
    // Identity + ownership come from the authenticated X-Api-Key, never the body.
    let caller = match caller.as_ref() {
        Some(c) => c,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // 1. Fetch folder ownership info
    let folder_query = sqlx::query!(
        "SELECT user_id, owner_api_key FROM folders WHERE id = $1",
        payload.folder_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder_query {
        Ok(Some(record)) => {
            // 2. Verify ownership
            if !caller.owns(&record.user_id, &record.owner_api_key) {
                return respond(404, "Folder not found", vec![], json!({}));
            }

            // 3. Remove ciphertext from R2 for every file in the WHOLE subtree
            // before the cascade drops the chunk rows. folders.parent_id and
            // files.folder_id cascade in Postgres, so the rows go automatically,
            // but the R2 objects would be orphaned forever without this (same
            // reason as the file permanent-delete). Walk the subtree, collect
            // each chunk id + its bucket, delete the objects, then delete the row.
            let objects = sqlx::query_as::<_, (String, Option<String>)>(
                "WITH RECURSIVE subtree AS ( \
                     SELECT id FROM folders WHERE id = $1 \
                     UNION ALL \
                     SELECT f.id FROM folders f JOIN subtree s ON f.parent_id = s.id \
                 ) \
                 SELECT c.id, fi.user_id FROM files fi \
                 JOIN chunks c ON c.file_id = fi.id \
                 WHERE fi.folder_id IN (SELECT id FROM subtree)",
            )
            .bind(&payload.folder_id)
            .fetch_all(&axum_state.pg_pool)
            .await
            .unwrap_or_default();
            for (chunk_id, user_id) in &objects {
                let storage = if user_id.is_some() { "sanctum" } else { "shadow" };
                if let Err(e) = axum_state.r2.delete_object(storage, chunk_id).await {
                    println!("[permanent-delete-folder] r2 delete {}/{} failed: {:?}", storage, chunk_id, e);
                }
            }

            // 4. Permanently delete the folder (descendant folders/files/chunks
            // cascade via FK).
            let delete_result = sqlx::query!(
                "DELETE FROM folders WHERE id = $1",
                payload.folder_id
            )
            .execute(&axum_state.pg_pool)
            .await;

            match delete_result {
                Ok(_) => respond(200, "Folder permanently deleted", vec![], json!({})),
                Err(_e) => respond(500, "Failed to delete folder", vec![], json!({})),
            }
        },
        Ok(None) => respond(404, "Folder not found", vec![], json!({})),
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}
