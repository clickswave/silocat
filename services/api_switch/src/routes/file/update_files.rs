use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub file_id: String,
    pub user_id: String,
    pub new_folder_id: Option<String>,
    // Potentially new_name later
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {

    // Verify ownership and update
    // Using COALESCE to only update fields that are provided
    let result = sqlx::query(
        "UPDATE files SET folder_id = $1 WHERE id = $2 AND owner_api_key = (SELECT api_key FROM users WHERE id = $3)",
    )
    .bind(payload.new_folder_id)
    .bind(payload.file_id)
    .bind(payload.user_id)
    .execute(&state.pg_pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                respond(200, "File updated", vec![], json!({}))
            } else {
                // If 0 rows, either file doesn't exist or user doesn't own it (api_key mismatch)
                 respond(404, "File not found or access denied", vec![], json!({}))
            }
        },
        Err(e) => {
             println!("Error updating file: {:?}", e);
             respond(500, "Failed to update file", vec![], json!({}))
        }
    }
}
