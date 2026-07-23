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

            // 3. Permanently delete
            // Note: This operation should ideally be recursive or cascade.
            // If cascade is set up in DB, deleting the folder row will delete children.
            // Assuming CASCADE ON DELETE for parent_id in folders and folder_id in files.
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
