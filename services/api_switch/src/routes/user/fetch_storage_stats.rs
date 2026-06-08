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
    
    // 1. Total storage limit = base (default_storage_bytes) + active, non-expired
    //    subscription space (promos / Pro grants live here, so they auto-expire).
    //    Runtime query (no macro) keeps the SQLX_OFFLINE prod build cache-free.
    let limit = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT ((SELECT default_storage_bytes FROM users WHERE id = $1) \
              + COALESCE((SELECT SUM(additional_space) FROM subscriptions \
                          WHERE created_by = $1 AND expires_on > NOW()), 0))::BIGINT"
    )
    .bind(&payload.user_id)
    .fetch_one(&state.pg_pool)
    .await;

    let default_storage = match limit {
        Ok(Some(total)) => total,
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
    
    // Only count COMPLETED uploads (uploaded_chunks >= total_chunks) so an
    // abandoned, half-uploaded file does not inflate the user's used quota.
    let usage = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(size), 0)::BIGINT FROM files          WHERE user_id = $1 AND deleted = false AND uploaded_chunks >= total_chunks"
    )
    .bind(&payload.user_id)
    .fetch_one(&state.pg_pool)
    .await;

    let used_bytes = match usage {
        Ok(v) => v.unwrap_or(0),
        Err(e) => {
             println!("Error calculating storage usage: {:?}", e);
             0 // Default to 0 on error for UI robustness
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
