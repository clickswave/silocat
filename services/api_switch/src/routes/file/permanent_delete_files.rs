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

            // 3. Permanently delete
            // Chunks should cascade if foreign keys are set up, otherwise we might need to delete them explicitly.
            // Assuming standard cascade or job cleanup.
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
