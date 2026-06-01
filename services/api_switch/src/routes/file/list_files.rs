use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub user_id: String,
    pub folder_id: Option<String>,
    pub starred: Option<bool>,
    pub shared: Option<bool>,
    pub deleted: Option<bool>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // Logic:
    // If deleted=true -> show deleted files (flat list usually, or per folder? Assuming flat list for trash view for now, or respect folder_id if provided)
    // Actually, trash view usually shows flat list of all deleted items or items in deleted folders. 
    // For simplicity: if deleted=true, we show all deleted files for the user (flat).
    
    let is_deleted = payload.deleted.unwrap_or(false);

    let files_result = if payload.starred == Some(true) {
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = $2 AND starred = true ORDER BY created_on DESC",
            payload.user_id,
            is_deleted
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else if payload.shared == Some(true) {
         sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = $2 AND share_type != 'off' ORDER BY created_on DESC",
            payload.user_id,
            is_deleted
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else if is_deleted {
        // Fetch all deleted files for the user if deleted=true is requested generally
        // Unless folder_id is specified? For trash view likely we want everything.
        // Let's assume flat list for now as per typical trash behavior.
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = true ORDER BY created_on DESC",
            payload.user_id
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else {
        // Normal folder view
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = false AND folder_id IS NOT DISTINCT FROM $2 ORDER BY created_on DESC",
            payload.user_id,
            payload.folder_id
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    };

    match files_result {
        Ok(files) => {
            respond(
                200,
                "Files retrieved successfully",
                vec![],
                json!({ "files": files }),
            )
        }
        Err(e) => {
            print!("Error fetching files: {:?}", e);
            respond(
                500,
                "Database error fetching files",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
