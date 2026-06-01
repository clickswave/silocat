use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize, Debug)]
pub struct PayloadBody {
    pub name: String,
    pub user_id: Option<String>,
    pub parent_id: Option<String>,
    pub uploaded_as_files: Option<bool>,
    pub owner_api_key: Option<String>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
    
    let uploaded_as_files = payload.uploaded_as_files.unwrap_or(false);

    let insert_query = sqlx::query_as!(
        models::Folder,
        "INSERT INTO folders (name, user_id, parent_id, uploaded_as_files, owner_api_key) 
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
        payload.name,
        payload.user_id,
        payload.parent_id,
        uploaded_as_files,
        payload.owner_api_key
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

    match insert_query {
        Ok(folder) => {
            respond(
                200,
                "Folder created",
                vec![],
                json!({ "folder": folder }),
            )
        },
        Err(e) => {
             respond(
                500,
                "Failed to create folder",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
