use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub folder_id: String,
    pub user_id: Option<String>,
    pub api_key: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {

    // Logic:
    // 1. If user_id is provided, delete where id = folder_id AND user_id = user_id
    // 2. If api_key is provided (and user_id is None or ignored for shadow), delete where id = folder_id AND owner_api_key = api_key
    
    let result = if let Some(uid) = payload.user_id {
        sqlx::query!(
            "UPDATE folders SET deleted = true, deleted_on = NOW() WHERE id = $1 AND user_id = $2",
            payload.folder_id,
            uid
        )
        .execute(&state.pg_pool)
        .await
    } else if let Some(key) = payload.api_key {
         sqlx::query!(
            "UPDATE folders SET deleted = true, deleted_on = NOW() WHERE id = $1 AND owner_api_key = $2",
            payload.folder_id,
            key
        )
        .execute(&state.pg_pool)
        .await
    } else {
        return respond(400, "Missing user_id or api_key", vec![], json!({}));
    };

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                respond(200, "Folder deleted", vec![], json!({}))
            } else {
                respond(404, "Folder not found or access denied", vec![], json!({}))
            }
        },
        Err(e) => {
            println!("Error deleting folder: {:?}", e);
            respond(500, "Failed to delete folder", vec![], json!({}))
        }
    }
}
