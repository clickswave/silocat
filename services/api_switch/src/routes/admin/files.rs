use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use futures_util::stream::{self, StreamExt};
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
    // Public files carry a share token; used to build a /s/<token> link so the
    // admin can open the normal (uncapped, chunk-reassembling) public download.
    pub share_token: Option<String>,
}

fn default_files_limit() -> i64 {
    100
}

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_files_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

pub async fn list_files(
    State(state): State<crate::AppState>,
    Query(p): Query<ListParams>,
) -> impl IntoResponse {
    // Server-side pagination so the admin panel can page past the first screen
    // of files instead of being capped at a hardcoded 100 with no way forward.
    let limit = p.limit.clamp(1, 500);
    let offset = p.offset.max(0);

    // Runtime query (not the compile-time macro) so adding pagination binds does
    // not require regenerating the sqlx offline cache.
    let files = sqlx::query_as::<_, File>(
        "SELECT id, user_id, name, mime, size, encrypted, created_on, downloads, public_access, share_token \
         FROM files ORDER BY created_on DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg_pool)
    .await;

    let total = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM files")
        .fetch_one(&state.pg_pool)
        .await
        .unwrap_or(0);

    match files {
        Ok(files) => respond(
            200,
            "Files retrieved successfully",
            vec![],
            json!({ "files": files, "total": total, "limit": limit, "offset": offset }),
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
    // Sanity ceiling only. The response is streamed chunk-by-chunk below, so peak
    // memory is one chunk regardless of file size and there is no reassembly
    // buffer to OOM the box. This guards an absurd/corrupt size row, not normal
    // large files (which used to fail at 200 MB with "File too large").
    const MAX_BYTES: i64 = 20 * 1024 * 1024 * 1024; // 20 GiB

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
            vec!["This file exceeds the admin download ceiling.".to_string()],
            json!({}),
        )
        .into_response();
    }

    // Owned files live in sanctum; anonymous files in shadow.
    let bucket = if user_id.is_some() { "sanctum" } else { "shadow" }.to_string();

    let chunk_rows = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM chunks WHERE file_id = $1 ORDER BY chunk_index ASC",
    )
    .bind(&id)
    .fetch_all(&state.pg_pool)
    .await;

    let chunk_ids: Vec<String> = match chunk_rows {
        Ok(c) => c.into_iter().map(|(cid,)| cid).collect(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    // Stream chunks straight to the client, fetching one at a time (`.then` is
    // sequential), so peak memory is a single chunk no matter how large the file.
    let body_stream = stream::iter(chunk_ids).then(move |cid| {
        let state = state.clone();
        let bucket = bucket.clone();
        async move {
            state
                .r2
                .get_object(&bucket, &cid)
                .await
                .map(Bytes::from)
                .map_err(|_e| std::io::Error::new(std::io::ErrorKind::Other, "storage error"))
        }
    });

    let disposition = format!("attachment; filename=\"{}\"", name.replace('"', ""));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, size.to_string())
        .body(Body::from_stream(body_stream))
        .unwrap()
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/{id}/download", get(download_file))
}
