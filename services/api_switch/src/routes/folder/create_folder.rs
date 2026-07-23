use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize, Debug)]
pub struct PayloadBody {
    pub name: String,
    pub parent_id: Option<String>,
    pub uploaded_as_files: Option<bool>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
    // Identity + ownership come from the authenticated X-Api-Key, never the body.
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

    let uploaded_as_files = payload.uploaded_as_files.unwrap_or(false);

    // If a parent is requested, it must belong to the caller.
    if let Some(parent_id) = &payload.parent_id {
        let parent_check = if let Some(uid) = &caller.user_id {
            sqlx::query_scalar!(
                "SELECT id FROM folders WHERE id = $1 AND user_id = $2",
                parent_id,
                uid
            )
            .fetch_optional(&axum_state.pg_pool)
            .await
        } else {
            sqlx::query_scalar!(
                "SELECT id FROM folders WHERE id = $1 AND owner_api_key = $2",
                parent_id,
                &caller.api_key
            )
            .fetch_optional(&axum_state.pg_pool)
            .await
        };

        match parent_check {
            Ok(Some(_)) => {}
            Ok(None) => {
                return respond(404, "Parent folder not found", vec![], json!({}))
            }
            Err(_e) => {
                return respond(500, "Database error", vec![], json!({}))
            }
        }
    }

    let insert_query = sqlx::query_as!(
        models::Folder,
        "INSERT INTO folders (name, user_id, parent_id, uploaded_as_files, owner_api_key)
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
        payload.name,
        caller.user_id.clone(),
        payload.parent_id,
        uploaded_as_files,
        Some(caller.api_key.clone())
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
        Err(_e) => {
             respond(
                500,
                "Failed to create folder",
                vec![],
                json!({}),
            )
        }
    }
}
