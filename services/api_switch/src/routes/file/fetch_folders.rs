use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub parent_id: Option<String>,
    pub starred: Option<bool>,
    pub shared: Option<bool>,
    pub deleted: Option<bool>,
}

// Define struct for query results (must match query columns exactly for query_as!)
#[derive(Debug)]
struct FolderRecord {
    id: String,
    name: String,
    created_on: chrono::DateTime<chrono::Utc>,
    parent_id: Option<String>,
    file_count: Option<i64>, // Count can be null if no rows? No, count(*) is usually i64. But let's use Option to be safe against older sqlx behavior or just unwrap later.
    starred: bool,
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

    let is_deleted = payload.deleted.unwrap_or(false);

    // We use query_as! to map directly to FolderRecord.
    // Note: We use `as "file_count"` to match the struct field.
    
    let folders_result: Result<Vec<FolderRecord>, sqlx::Error> = if payload.starred == Some(true) {
        // Starred folders
         sqlx::query_as!(
            FolderRecord,
            r#"
            SELECT 
                f.id, 
                f.name, 
                f.created_on, 
                f.parent_id,
                f.starred,
                ((SELECT COUNT(*) FROM files WHERE folder_id = f.id AND user_id = $1 AND deleted = $2) + (SELECT COUNT(*) FROM folders WHERE parent_id = f.id AND user_id = $1 AND deleted = $2)) as "file_count"
            FROM folders f
            WHERE f.user_id = $1 AND f.starred = true AND f.deleted = $2
            ORDER BY f.created_on DESC
            "#,
            user_id,
            is_deleted
        )
        .fetch_all(&state.pg_pool)
        .await

    } else if payload.shared == Some(true) {
        // Shared folders
         sqlx::query_as!(
            FolderRecord,
            r#"
            SELECT 
                f.id, 
                f.name, 
                f.created_on, 
                f.parent_id,
                f.starred,
                ((SELECT COUNT(*) FROM files WHERE folder_id = f.id AND user_id = $1 AND deleted = $2) + (SELECT COUNT(*) FROM folders WHERE parent_id = f.id AND user_id = $1 AND deleted = $2)) as "file_count"
            FROM folders f
            WHERE f.user_id = $1 AND f.share_type != 'off' AND f.deleted = $2
            ORDER BY f.created_on DESC
            "#,
            user_id,
            is_deleted
        )
        .fetch_all(&state.pg_pool)
        .await

    } else if is_deleted {
         sqlx::query_as!(
            FolderRecord,
            r#"
            SELECT 
                f.id, 
                f.name, 
                f.created_on, 
                f.parent_id,
                f.starred,
                ((SELECT COUNT(*) FROM files WHERE folder_id = f.id AND user_id = $1 AND deleted = false) + (SELECT COUNT(*) FROM folders WHERE parent_id = f.id AND user_id = $1 AND deleted = false)) as "file_count"
            FROM folders f
            WHERE f.user_id = $1 AND f.deleted = true
            ORDER BY f.created_on DESC
            "#,
            user_id
        )
        .fetch_all(&state.pg_pool)
        .await

    } else if let Some(pid) = payload.parent_id {
         sqlx::query_as!(
            FolderRecord,
            r#"
            SELECT 
                f.id, 
                f.name, 
                f.created_on, 
                f.parent_id,
                f.starred,
                ((SELECT COUNT(*) FROM files WHERE folder_id = f.id AND user_id = $1 AND deleted = false) + (SELECT COUNT(*) FROM folders WHERE parent_id = f.id AND user_id = $1 AND deleted = false)) as "file_count"
            FROM folders f
            WHERE f.user_id = $1 AND f.parent_id = $2 AND f.deleted = false
            ORDER BY f.created_on DESC
            "#,
            user_id,
            pid
        )
        .fetch_all(&state.pg_pool)
        .await
        
    } else {
         sqlx::query_as!(
            FolderRecord,
            r#"
            SELECT 
                f.id, 
                f.name, 
                f.created_on, 
                f.parent_id,
                f.starred,
                ((SELECT COUNT(*) FROM files WHERE folder_id = f.id AND user_id = $1 AND deleted = false) + (SELECT COUNT(*) FROM folders WHERE parent_id = f.id AND user_id = $1 AND deleted = false)) as "file_count"
            FROM folders f
            WHERE f.user_id = $1 AND f.parent_id IS NULL AND f.deleted = false
            ORDER BY f.created_on DESC
            "#,
            user_id
        )
        .fetch_all(&state.pg_pool)
        .await
    };

    match folders_result {
        Ok(folders) => {
            let data: Vec<serde_json::Value> = folders.iter().map(|f| json!({
                "id": f.id,
                "name": f.name,
                "created_on": f.created_on,
                "parent_id": f.parent_id,
                "count": f.file_count.unwrap_or(0),
                "starred": f.starred
            })).collect();
            
            respond(200, "Folders fetched", vec![], json!({ "folders": data }))
        }
        Err(e) => {
            println!("Error fetching folders: {:?}", e);
            respond(500, "Failed to fetch folders", vec![], json!({}))
        }
    }
}
