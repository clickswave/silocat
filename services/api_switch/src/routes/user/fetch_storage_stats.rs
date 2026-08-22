use axum::{extract::State, Extension, response::IntoResponse};
use serde::Serialize;
use serde_json::json;
use crate::{models::UserTokenData, routes::respond};

#[derive(Serialize, Debug)]
pub struct StorageStats {
    pub total: i64,
    pub used: i64,
    pub free: i64,
}

// Identity comes from the authenticated X-Api-Key (validate_token), NEVER a body
// `user_id`. Previously this was a public route that returned any user's stats
// for an attacker-supplied `user_id`; now it can only report the caller's own.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
) -> impl IntoResponse {
    let user_id = user.id;

    // 1. Total storage limit = base (default_storage_bytes) + active, non-expired
    //    subscription space (promos / Pro grants live here, so they auto-expire).
    //    Runtime query (no macro) keeps the SQLX_OFFLINE prod build cache-free.
    let limit = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT ((SELECT default_storage_bytes FROM users WHERE id = $1) \
              + COALESCE((SELECT SUM(additional_space) FROM subscriptions \
                          WHERE created_by = $1 AND expires_on > NOW()), 0))::BIGINT"
    )
    .bind(&user_id)
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

    // 2. Used storage = real stored bytes (each uploaded chunk's HeadObject
    //    size_on_server) for non-deleted files. Matches libs::quota so the UI
    //    counter and the upload gate agree, and can't be gamed by a client that
    //    under-declares files.size.
    let usage = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(c.size_on_server), 0)::BIGINT \
         FROM chunks c JOIN files f ON f.id = c.file_id \
         WHERE f.user_id = $1 AND f.deleted = false AND c.uploaded = true"
    )
    .bind(&user_id)
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
