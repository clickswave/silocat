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

#[derive(serde::Serialize)]
pub struct ChunkResponse {
    pub id: String,
    pub index: i32,
    pub size: i64,
    pub presigned_url: String, // Download URL
    pub file_offset: i64,
    pub checksum: String,
    pub nonce: Option<String>,
    pub salt: Option<String>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Get file info to determine storage bucket (shadow/sanctum)
    let file_query = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;
    
    let file = match file_query {
        Ok(Some(f)) => f,
        Ok(None) => return respond(404, "File not found", vec![], json!({})),
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
    };

    let storage_type = if file.user_id.is_some() { "sanctum" } else { "shadow" };

    // 2. Fetch all chunks
    let chunks_query = sqlx::query_as::<_, models::Chunk>(
        "SELECT * FROM chunks WHERE file_id = $1 AND uploaded = true ORDER BY chunk_index ASC",
    )
    .bind(payload.file_id)
    .fetch_all(&axum_state.pg_pool)
    .await;

    match chunks_query {
        Ok(chunks) => {
            let mut response_chunks = Vec::new();

            for chunk in chunks {
                // Generate presigned GET url
                match axum_state.r2.presigned_get_url(storage_type, &chunk.id).await {
                    Ok(url) => {
                        response_chunks.push(ChunkResponse {
                            id: chunk.id,
                            index: chunk.chunk_index,
                            size: chunk.size,
                            presigned_url: url,
                            file_offset: chunk.file_offset,
                            checksum: chunk.checksum,
                            nonce: chunk.nonce,
                            salt: chunk.salt,
                        });
                    },
                    Err(e) => {
                        return respond(500, "Failed to generate download URL", vec![e.to_string()], json!({}));
                    }
                }
            }
            
            respond(
                200,
                "Chunks retrieved",
                vec![],
                json!({ "chunks": response_chunks }),
            )
        },
        Err(e) => {
             respond(
                500,
                "Failed to fetch chunks",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
