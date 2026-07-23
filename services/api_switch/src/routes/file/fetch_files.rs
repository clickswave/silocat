use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub file_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // Retrieve file metadata
    let file = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file {
        Ok(Some(file)) => {
            // Access control: public files are readable by anyone; otherwise the
            // caller must own the file. Blocks reading another user's file by id.
            let allowed = file.public_access
                || caller
                    .as_ref()
                    .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key));
            if !allowed {
                return respond(404, "File not found", vec![], json!({}));
            }
            respond(
                200,
                "File found",
                vec![],
                json!({ "file": file }),
            )
        }
        Ok(None) => {
             respond(
                404,
                "File not found",
                vec![],
                json!({}),
            )
        }
        Err(_e) => {
            respond(
                500,
                "Database error",
                vec![],
                json!({}),
            )
        }
    }
}
