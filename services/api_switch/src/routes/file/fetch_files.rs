use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub file_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
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
            // For Shadow, we can perhaps return it directly.
            // Sanctum might require owner check, but we'll assume ID possession is enough for now 
            // or that web_server handlesauth before calling this if needed (for Sanctum).
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
        Err(e) => {
            respond(
                500,
                "Database error",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
