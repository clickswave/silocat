use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    chunk_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
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

    // 0. Resolve the chunk's owning file and verify the caller owns it before
    //    touching any upload state (no marking another caller's chunks complete).
    let owner = sqlx::query!(
        "SELECT f.id as file_id, f.user_id, f.owner_api_key
         FROM chunks c JOIN files f ON f.id = c.file_id
         WHERE c.id = $1",
        payload.chunk_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    let file_id = match owner {
        Ok(Some(rec)) => {
            if !caller.owns(&rec.user_id, &rec.owner_api_key) {
                return respond(404, "Chunk not found", vec![], json!({}));
            }
            rec.file_id
        }
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    // 1. Mark the chunk uploaded.
    if let Err(_e) = sqlx::query!(
        "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = size WHERE id = $1",
        payload.chunk_id
    )
    .execute(&axum_state.pg_pool)
    .await
    {
        return respond(500, "Failed to update chunk status", vec![], json!({}));
    }

    // 2. Recount completed chunks and update the file.
    let count_query = sqlx::query!(
        "SELECT COUNT(*) as count FROM chunks WHERE file_id = $1 AND uploaded = true",
        file_id
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

    match count_query {
        Ok(record) => {
            let _ = sqlx::query!(
                "UPDATE files SET uploaded_chunks = $1 WHERE id = $2",
                record.count.unwrap_or(0) as i32,
                file_id
            )
            .execute(&axum_state.pg_pool)
            .await;
        }
        Err(e) => {
            println!("Failed to update file chunk count: {}", e);
        }
    }

    respond(200, "Chunk marked as complete", vec![], json!({}))
}
