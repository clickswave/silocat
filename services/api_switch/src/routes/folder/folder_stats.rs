use axum::{extract::State, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub user_id: String,
    pub folder_id: String,
}

#[derive(Serialize, Debug)]
pub struct FolderStats {
    pub total_items: i64,
    pub folders: i64,
    pub files: i64,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Recursive CTE to count files and folders
    let query_result = sqlx::query!(
        r#"
        WITH RECURSIVE folder_tree AS (
            -- Base case: the target folder
            SELECT id
            FROM folders
            WHERE id = $1 AND user_id = $2
            
            UNION ALL
            
            -- Recursive step: subfolders
            SELECT f.id
            FROM folders f
            INNER JOIN folder_tree ft ON f.parent_id = ft.id
            WHERE f.user_id = $2
        )
        SELECT 
            (SELECT COUNT(*) FROM folder_tree) - 1 as "folder_count!", -- Subtract 1 to exclude the root folder itself from "items inside"
            (SELECT COUNT(*) FROM files WHERE folder_id IN (SELECT id FROM folder_tree) AND user_id = $2) as "file_count!"
        "#,
        payload.folder_id,
        payload.user_id
    )
    .fetch_one(&state.pg_pool)
    .await;

    match query_result {
        Ok(record) => {
            let folders = record.folder_count;
            let files = record.file_count;
            let total_items = folders + files;

            respond(
                200,
                "Stats calculated",
                vec![],
                json!({
                    "data": {
                        "total_items": total_items,
                        "folders": folders,
                        "files": files
                    }
                }),
            )
        }
        Err(e) => {
            println!("Error calculating folder stats: {:?}", e);
            respond(500, "Failed to calculate stats", vec![e.to_string()], json!({}))
        }
    }
}
