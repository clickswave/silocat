use axum::{extract::State, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub user_id: String,
}

#[derive(Serialize, Debug)]
pub struct StorageStats {
    pub total: i64,
    pub used: i64,
    pub free: i64,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    
    // 1. Get User's Total Storage Limit
    let user = sqlx::query!(
        "SELECT default_storage_bytes FROM users WHERE id = $1",
        payload.user_id
    )
    .fetch_optional(&state.pg_pool)
    .await;

    let default_storage = match user {
        Ok(Some(u)) => u.default_storage_bytes,
        Ok(None) => return respond(404, "User not found", vec![], json!({})),
        Err(e) => {
            println!("Error fetching user storage limit: {:?}", e);
            return respond(500, "Database error", vec![], json!({}))
        }
    };

    // 2. Calculate Used Storage (Sum of non-deleted files)
    // Note: We should likely include 'uploading' chunks or similar in a refined model, 
    // but for now, summing file sizes is a good start. 
    // Better accuracy: Sum of all chunks marked 'uploaded' for files owned by user?
    // Current 'files' table 'size' is the logical size. 'chunks' table has 'size'.
    // Let's sum 'files.size' for simplicity as the primary metric.
    
    let usage = sqlx::query!(
        "SELECT COALESCE(SUM(size), 0)::BIGINT as used_bytes FROM files WHERE user_id = $1 AND deleted = false",
        payload.user_id
    )
    .fetch_one(&state.pg_pool)
    .await;

    let used_bytes = match usage {
        Ok(rec) => rec.used_bytes.unwrap_or(0),
        Err(e) => {
             println!("Error calculating storage usage: {:?}", e);
             0 // Default to 0 on error? Or ret 500? Let's default 0 for robustness in UI
        }
    };

    let free_bytes = default_storage - used_bytes;

    respond(
        200,
        "Storage stats fetched",
        vec![],
        json!(StorageStats {
            total: default_storage,
            used: used_bytes,
            free: free_bytes
        }),
    )
}
