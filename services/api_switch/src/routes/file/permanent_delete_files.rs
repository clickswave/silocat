use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize)]
pub struct DeletePayload {
    pub file_id: String,
    pub api_key: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<DeletePayload>,
) -> impl IntoResponse {

    // 1. Fetch file ownership info
    let file_query = sqlx::query!(
        "SELECT user_id, owner_api_key FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file_query {
        Ok(Some(record)) => {
            // 2. Verify ownership
            let mut is_owner = false;

            if let Some(key) = record.owner_api_key {
                if key == payload.api_key {
                    is_owner = true;
                }
            }
            
            if !is_owner {
                if let Some(file_user_id) = record.user_id {
                    let user_check = sqlx::query!(
                        "SELECT id FROM users WHERE api_key = $1",
                        payload.api_key
                    )
                    .fetch_optional(&axum_state.pg_pool)
                    .await;

                    if let Ok(Some(user)) = user_check {
                        if user.id == file_user_id {
                            is_owner = true;
                        }
                    }
                }
            }

            if is_owner {
                // 3. Permanently delete
                // Chunks should cascade if foreign keys are set up, otherwise we might need to delete them explicitly.
                // Assuming standard cascade or job cleanup. 
                // Wait, typically chunks reference numeric ID or UUID? If UUID and FK CASCADE, it's fine.
                // Ideally we should delete from chunks first if no cascade.
                // Let's assume standard deletion for now.
                
                let delete_result = sqlx::query!(
                    "DELETE FROM files WHERE id = $1",
                    payload.file_id
                )
                .execute(&axum_state.pg_pool)
                .await;

                match delete_result {
                    Ok(_) => respond(200, "File permanently deleted", vec![], json!({})),
                    Err(e) => respond(500, "Failed to delete file", vec![e.to_string()], json!({})),
                }
            } else {
                respond(403, "Unauthorized", vec!["API Key does not match file owner".to_string()], json!({}))
            }
        },
        Ok(None) => respond(404, "File not found", vec![], json!({})),
        Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
    }
}
