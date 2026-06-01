use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub folder_id: String,
    pub user_id: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {

    // Ideally, we should soft delete subfolders/files too, or check if empty.
    // For now, let's just delete the folder record itself (soft delete preferred if schema supports it, but checking schema first might be good).
    // Assuming 'deleted' column exists on folders table like files table.
    
    // Check if folder belongs to user
    let folder = sqlx::query!(
        "SELECT id FROM folders WHERE id = $1 AND user_id = $2",
        payload.folder_id,
        payload.user_id
    )
    .fetch_optional(&state.pg_pool)
    .await;

    match folder {
        Ok(Some(_)) => {
            // Found, proceed to delete
            // Soft delete: mark as deleted
             let result = sqlx::query!(
                "UPDATE folders SET deleted = true, deleted_on = NOW() WHERE id = $1",
                payload.folder_id
            )
            .execute(&state.pg_pool)
            .await;

            match result {
                Ok(_) => respond(200, "Folder deleted", vec![], json!({})),
                Err(e) => {
                    println!("Error deleting folder: {:?}", e);
                    respond(500, "Failed to delete folder", vec![], json!({}))
                }
            }
        },
        Ok(None) => respond(404, "Folder not found or access denied", vec![], json!({})),
        Err(e) => {
            println!("Error checking folder ownership: {:?}", e);
             respond(500, "Internal Server Error", vec![], json!({}))
        }
    }
}
