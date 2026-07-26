use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub name: String,
    pub parent_id: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Owner is the authenticated caller (user or shadow): never a body field.
    let caller = match caller.as_ref() {
        Some(c) => c,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // If nesting under a parent, the parent must belong to the caller.
    if let Some(parent_id) = &payload.parent_id {
        let parent = sqlx::query!(
            "SELECT user_id, owner_api_key FROM folders WHERE id = $1",
            parent_id
        )
        .fetch_optional(&state.pg_pool)
        .await;
        match parent {
            Ok(Some(p)) if caller.owns(&p.user_id, &p.owner_api_key) => {}
            Ok(_) => return respond(404, "Parent folder not found", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }
    }

    let folder = sqlx::query!(
        "INSERT INTO folders (name, user_id, parent_id, owner_api_key) VALUES ($1, $2, $3, $4) RETURNING id, name, created_on",
        payload.name,
        caller.user_id,
        payload.parent_id,
        Some(caller.api_key.clone()),
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
