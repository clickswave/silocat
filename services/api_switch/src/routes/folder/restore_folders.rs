use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize)]
pub struct RestorePayload {
    pub folder_id: String,
    pub api_key: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<RestorePayload>,
) -> impl IntoResponse {

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
            let mut is_owner = false;

            if let Some(key) = record.owner_api_key {
                if key == payload.api_key {
                    is_owner = true;
                }
            }
            
            if !is_owner {
                if let Some(folder_user_id) = record.user_id {
                    let user_check = sqlx::query!(
                        "SELECT id FROM users WHERE api_key = $1",
                        payload.api_key
                    )
                    .fetch_optional(&axum_state.pg_pool)
                    .await;

                    if let Ok(Some(user)) = user_check {
                        if user.id == folder_user_id {
                            is_owner = true;
                        }
                    }
                }
            }

            if is_owner {
                // 3. Mark as not deleted
                let restore_result = sqlx::query!(
                    "UPDATE folders SET deleted = false, deleted_on = NULL WHERE id = $1",
                    payload.folder_id
                )
                .execute(&axum_state.pg_pool)
                .await;

                match restore_result {
                    Ok(_) => respond(200, "Folder restored successfully", vec![], json!({})),
                    Err(e) => respond(500, "Failed to restore folder", vec![e.to_string()], json!({})),
                }
            } else {
                respond(403, "Unauthorized", vec!["API Key does not match folder owner".to_string()], json!({}))
            }
        },
        Ok(None) => respond(404, "Folder not found", vec![], json!({})),
        Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
    }
}
