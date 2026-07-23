use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

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
    // Shadow (anonymous) sessions may delete their own folders, so any caller is allowed.
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
    .fetch_optional(&state.pg_pool)
    .await;

    match folder_query {
        Ok(Some(record)) => {
            // 2. Verify ownership. Return 404 (not 403) on non-ownership to avoid
            // leaking the existence of another caller's folder.
            if !caller.owns(&record.user_id, &record.owner_api_key) {
                return respond(404, "Folder not found", vec![], json!({}));
            }

            // 3. Mark as deleted
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
        }
        Ok(None) => respond(404, "Folder not found", vec![], json!({})),
        Err(e) => {
            println!("Error deleting folder: {:?}", e);
            respond(500, "Failed to delete folder", vec![], json!({}))
        }
    }
}
