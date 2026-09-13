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

    // Same rule as create_files.rs, and as folder/create_folder.rs: decided from
    // the caller, never from the body. False for a drive folder, which is
    // published by the share sheet and nothing else; true for an anonymous drop,
    // which has no drive to sit in and no owner who can log in later to publish
    // it, so the id IS the capability.
    //
    // This is the handler the web app actually reaches (`/file/create-folders`,
    // via ApiServerRoutes.createFolder). The near-identical one behind
    // `/folder/create` is the one that reads like the obvious place for this, so
    // fixing only that left every folder drop made through the UI private while
    // the by-id read path and the 0045 backfill both looked correct.
    let public_access = caller.user_id.is_none();

    let folder = sqlx::query!(
        "INSERT INTO folders (name, user_id, parent_id, owner_api_key, public_access) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id, name, created_on",
        payload.name,
        caller.user_id,
        payload.parent_id,
        Some(caller.api_key.clone()),
        public_access,
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
