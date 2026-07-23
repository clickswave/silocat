use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub folder_id: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    let user_id = match caller.as_ref().and_then(|c| c.user_id.clone()) {
        Some(uid) => uid,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // Check if folder belongs to user
    let folder = sqlx::query!(
        "SELECT id FROM folders WHERE id = $1 AND user_id = $2",
        payload.folder_id,
        user_id
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
