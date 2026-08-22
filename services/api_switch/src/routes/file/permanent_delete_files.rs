use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

#[derive(Deserialize)]
pub struct DeletePayload {
    pub file_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<DeletePayload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    // Shadow (anonymous) sessions may permanently delete their own files, so any caller is allowed.
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

    // 1. Fetch file ownership info
    let file_query = sqlx::query!(
        "SELECT user_id, owner_api_key FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file_query {
        Ok(Some(record)) => {
            // 2. Verify ownership. Return 404 (not 403) on non-ownership to avoid
            // leaking the existence of another caller's file.
            if !caller.owns(&record.user_id, &record.owner_api_key) {
                return respond(404, "File not found", vec![], json!({}));
            }

            // 3. Remove the ciphertext from R2 BEFORE the DB rows. The chunk rows
            // cascade away on the file delete, so once they are gone nothing can
            // ever rediscover the object keys; skipping this leaves the encrypted
            // bytes in R2 forever (a privacy contradiction for a zero-knowledge
            // product, and a monotonic storage-cost leak). Mirrors watchcat's
            // gc_expired_trash; delete_object is idempotent. R2 key = chunk id.
            let storage = if record.user_id.is_some() { "sanctum" } else { "shadow" };
            let chunk_ids: Vec<String> =
                sqlx::query_scalar("SELECT id FROM chunks WHERE file_id = $1")
                    .bind(&payload.file_id)
                    .fetch_all(&axum_state.pg_pool)
                    .await
                    .unwrap_or_default();
            for cid in &chunk_ids {
                if let Err(e) = axum_state.r2.delete_object(storage, cid).await {
                    println!("[permanent-delete] r2 delete {}/{} failed: {:?}", storage, cid, e);
                }
            }

            // 4. Permanently delete the DB rows (chunk rows cascade via FK).
            let delete_result = sqlx::query!(
                "DELETE FROM files WHERE id = $1",
                payload.file_id
            )
            .execute(&axum_state.pg_pool)
            .await;

            match delete_result {
                Ok(_) => respond(200, "File permanently deleted", vec![], json!({})),
                Err(_e) => respond(500, "Failed to delete file", vec![], json!({})),
            }
        },
        Ok(None) => respond(404, "File not found", vec![], json!({})),
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}
