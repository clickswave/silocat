use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub name: String,
    pub user_id: Option<String>,
    pub parent_id: Option<String>,
    pub owner_api_key: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {

    let folder = sqlx::query!(
        "INSERT INTO folders (name, user_id, parent_id, owner_api_key) VALUES ($1, $2, $3, $4) RETURNING id, name, created_on",
        payload.name,
        payload.user_id,
        payload.parent_id,
        payload.owner_api_key
    )
    .fetch_one(&state.pg_pool)
    .await;

    match folder {
        Ok(f) => respond(201, "Folder created", vec![], json!({
            "id": f.id,
            "name": f.name,
            "created_on": f.created_on
        })),
        Err(e) => {
            println!("Error creating folder: {:?}", e);
            respond(500, "Failed to create folder", vec![], json!({}))
        }
    }
}
