use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    chunk_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Update chunk status
    let update_chunk_query = sqlx::query!(
        "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = size 
         WHERE id = $1 RETURNING file_id",
        payload.chunk_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    let file_id = match update_chunk_query {
        Ok(Some(record)) => record.file_id,
        Ok(None) => {
             return respond(
                404,
                "Chunk not found",
                vec![],
                json!({}),
            );
        }
        Err(e) => {
            return respond(
                500,
                "Failed to update chunk status",
                vec![e.to_string()],
                json!({}),
            );
        }
    };

    // 2. Increment file uploaded_chunks count
    // Ideally this should be a transaction or a count query, but for now simple increment is fine explicitly
    // Actually, safer to count completed chunks
    let count_query = sqlx::query!(
        "SELECT COUNT(*) as count FROM chunks WHERE file_id = $1 AND uploaded = true",
        file_id
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

     match count_query {
        Ok(record) => {
             // Update file uploaded_chunks
             let _ = sqlx::query!(
                "UPDATE files SET uploaded_chunks = $1 WHERE id = $2",
                record.count.unwrap_or(0) as i32,
                file_id
             ).execute(&axum_state.pg_pool).await;
        },
        Err(e) => {
             // log error but don't fail the request significantly as chunk is marked
             println!("Failed to update file chunk count: {}", e);
        }
    }

    respond(
        200,
        "Chunk marked as complete",
        vec![],
        json!({}),
    )
}
