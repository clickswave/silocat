use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub folder_id: Option<String>,
    pub starred: Option<bool>,
    pub shared: Option<bool>,
    pub deleted: Option<bool>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    // The sanctum file browser is for registered users.
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

    let files_result = if payload.starred == Some(true) {
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = $2 AND starred = true ORDER BY created_on DESC",
            user_id,
            is_deleted
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else if payload.shared == Some(true) {
         sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = $2 AND share_type != 'off' ORDER BY created_on DESC",
            user_id,
            is_deleted
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else if is_deleted {
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = true ORDER BY created_on DESC",
            user_id
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    } else {
        sqlx::query_as!(
            models::File,
            "SELECT * FROM files WHERE user_id = $1 AND deleted = false AND folder_id IS NOT DISTINCT FROM $2 ORDER BY created_on DESC",
            user_id,
            payload.folder_id
        )
        .fetch_all(&axum_state.pg_pool)
        .await
    };

    match files_result {
        Ok(files) => respond(
            200,
            "Files retrieved successfully",
            vec![],
            json!({ "files": files }),
        ),
        Err(_e) => respond(
            500,
            "Database error fetching files",
            vec![],
            json!({}),
        ),
    }
}
