use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, types::chrono::{DateTime, Utc}};
use crate::routes::respond;

#[derive(Serialize, Deserialize, Clone, Debug, FromRow)]
pub struct File {
    pub id: String,
    pub user_id: Option<String>,
    pub name: String,
    pub mime: String,
    pub size: i64,
    pub encrypted: bool,
    pub created_on: DateTime<Utc>,
    pub downloads: i64,
    pub public_access: bool,
}

pub async fn list_files(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let files = sqlx::query_as!(
        File,
        "SELECT id, user_id, name, mime, size, encrypted, created_on, downloads, public_access FROM files ORDER BY created_on DESC LIMIT 100"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match files {
        Ok(files) => respond(
            200,
            "Files retrieved successfully",
            vec![],
            json!({ "files": files }),
        ),
        Err(_e) => respond(
            500,
            "Failed to retrieve files",
            vec![],
            json!({}),
        ),
    }
}

/// GET /admin/files/{id}/download: reassemble the file from its chunks and
/// stream it as an attachment. Only for UNENCRYPTED files (E2E-encrypted files
/// are stored as ciphertext the server can't decrypt). Size-capped to keep the
/// admin box from OOMing on huge files.
async fn download_file(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Response {
    const MAX_BYTES: i64 = 200 * 1024 * 1024;

    let row = sqlx::query_as::<_, (Option<String>, String, String, i64, bool)>(
        "SELECT user_id, name, mime, size, encrypted FROM files WHERE id = $1 AND deleted = false",
    )
    .bind(&id)
    .fetch_optional(&state.pg_pool)
    .await;

    let (user_id, name, mime, size, encrypted) = match row {
        Ok(Some(r)) => r,
        Ok(None) => return respond(404, "File not found", vec![], json!({})).into_response(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    if encrypted {
        return respond(
            400,
            "File is encrypted",
            vec!["This file is end-to-end encrypted; the server cannot produce a decrypted download.".to_string()],
            json!({}),
        )
        .into_response();
    }
    if size > MAX_BYTES {
        return respond(
            413,
            "File too large",
            vec!["This file is too large to download from the admin panel.".to_string()],
            json!({}),
        )
        .into_response();
    }

    // Owned files live in sanctum; anonymous files in shadow.
    let bucket = if user_id.is_some() { "sanctum" } else { "shadow" };

    let chunk_rows = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM chunks WHERE file_id = $1 ORDER BY chunk_index ASC",
    )
    .bind(&id)
    .fetch_all(&state.pg_pool)
    .await;

    let chunk_ids = match chunk_rows {
        Ok(c) => c,
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    let mut buf: Vec<u8> = Vec::with_capacity(size.max(0) as usize);
    for (cid,) in chunk_ids {
        match state.r2.get_object(bucket, &cid).await {
            Ok(mut bytes) => buf.append(&mut bytes),
            Err(_e) => {
                return respond(502, "Storage error", vec![], json!({})).into_response()
            }
        }
    }

    let disposition = format!("attachment; filename=\"{}\"", name.replace('"', ""));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(Body::from(buf))
        .unwrap()
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/{id}/download", get(download_file))
}
