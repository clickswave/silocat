use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub file_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // Retrieve file metadata
    let file = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file {
        Ok(Some(file)) => {
            // Access control. The owner always reaches their own file (incl.
            // trashed) via `owns`. A non-owner may read a file only through the
            // public_access shortcut, and ONLY when the file carries no active
            // share protection: a password, a one-time link, an expiry, or a
            // soft-delete must force the token-checked /file/public/share/* path,
            // never leak by id.
            let now = chrono::Utc::now();
            let share_protected = file.share_password_hash.is_some()
                || file.share_type.as_deref() == Some("once")
                || file.share_expires_at.map_or(false, |e| e <= now)
                || file.deleted;
            let allowed = caller
                .as_ref()
                .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key))
                || (file.public_access && !share_protected);
            if !allowed {
                return respond(404, "File not found", vec![], json!({}));
            }
            respond(
                200,
                "File found",
                vec![],
                json!({ "file": file }),
            )
        }
        Ok(None) => {
             respond(
                404,
                "File not found",
                vec![],
                json!({}),
            )
        }
        Err(_e) => {
            respond(
                500,
                "Database error",
                vec![],
                json!({}),
            )
        }
    }
}
