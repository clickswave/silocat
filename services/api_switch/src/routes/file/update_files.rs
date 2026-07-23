use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub file_id: String,
    pub new_folder_id: Option<String>,
    // Potentially new_name later
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

    // If moving into a folder, verify that destination folder belongs to the caller.
    if let Some(ref new_folder_id) = payload.new_folder_id {
        let folder = sqlx::query("SELECT id FROM folders WHERE id = $1 AND user_id = $2")
            .bind(new_folder_id)
            .bind(&user_id)
            .fetch_optional(&state.pg_pool)
            .await;

        match folder {
            Ok(Some(_)) => {}
            Ok(None) => {
                return respond(404, "Destination folder not found", vec![], json!({}))
            }
            Err(_e) => {
                return respond(
                    500,
                    "Failed to update file",
                    vec![],
                    json!({}),
                )
            }
        }
    }

    // Update, scoped to the caller's own files.
    let result = sqlx::query(
        "UPDATE files SET folder_id = $1 WHERE id = $2 AND user_id = $3",
    )
    .bind(payload.new_folder_id)
    .bind(payload.file_id)
    .bind(user_id)
    .execute(&state.pg_pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                respond(200, "File updated", vec![], json!({}))
            } else {
                // If 0 rows, either file doesn't exist or the caller doesn't own it.
                respond(404, "File not found or access denied", vec![], json!({}))
            }
        }
        Err(_) => respond(500, "Failed to update file", vec![], json!({})),
    }
}
