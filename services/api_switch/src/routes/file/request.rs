//! "Request a file": the inbound half of secure delivery.
//!
//! An account owner creates a request link; anyone with the link can upload files
//! that land in the owner's account. These handlers are ISOLATED from the normal
//! upload path (`create_files` / `mark_chunk_complete`): the request upload
//! pipeline lives entirely here and is authorized by the request token, so the
//! live upload flow (which the anonymous-drop traffic depends on) is untouched.
//! The uploaded file is an ordinary `files` row owned by the requester;
//! `request_uploads` ties it to the request.

use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::{Extension, Json};
use chrono::Utc;
use serde_json::json;

// ===========================================================================
// Owner side (a logged-in account)
// ===========================================================================

#[derive(serde::Deserialize)]
pub struct CreateRequestPayload {
    pub label: Option<String>,
    pub message: Option<String>,
    pub folder_id: Option<String>,
    pub expires_in_days: Option<i64>,
    pub max_uploads: Option<i32>,
}

fn require_account(caller: &Option<Caller>) -> Result<String, axum::response::Response> {
    match caller.as_ref().and_then(|c| c.user_id.clone()) {
        Some(u) => Ok(u),
        None => Err(respond(
            401,
            "Unauthorized",
            vec!["A logged-in account is required.".to_string()],
            json!({}),
        )
        .into_response()),
    }
}

/// Create a file request for the caller's account. Returns the request id + token.
pub async fn create_request(
    State(st): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(p): Json<CreateRequestPayload>,
) -> impl IntoResponse {
    let owner = match require_account(&caller) {
        Ok(u) => u,
        Err(r) => return r,
    };

    // Request-a-file is a paid delivery feature.
    if !crate::libs::plans::is_paid(&st.pg_pool, &owner).await {
        return respond(
            403,
            "Upgrade required",
            vec!["Requesting files is a paid feature. Upgrade to receive files straight into your account.".to_string()],
            json!({ "upgrade_required": true, "feature": "request_a_file" }),
        )
        .into_response();
    }

    // An optional destination folder must belong to the owner.
    if let Some(fid) = &p.folder_id {
        let f = sqlx::query!("SELECT user_id FROM folders WHERE id = $1", fid)
            .fetch_optional(&st.pg_pool)
            .await;
        match f {
            Ok(Some(rec)) if rec.user_id.as_deref() == Some(owner.as_str()) => {}
            _ => return respond(404, "Destination folder not found", vec![], json!({})).into_response(),
        }
    }

    let token = format!("{}{}", crate::libs::rng::uuid(), crate::libs::rng::uuid()).replace('-', "");
    let expires_at = p
        .expires_in_days
        .filter(|d| *d > 0)
        .map(|d| Utc::now() + chrono::Duration::days(d));

    let row = sqlx::query!(
        "INSERT INTO file_requests (token, owner_user_id, label, message, folder_id, max_uploads, expires_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING id, token",
        token,
        owner,
        p.label,
        p.message,
        p.folder_id,
        p.max_uploads,
        expires_at
    )
    .fetch_one(&st.pg_pool)
    .await;

    match row {
        Ok(r) => respond(
            201,
            "Request created",
            vec![],
            json!({ "id": r.id, "token": r.token }),
        )
        .into_response(),
        Err(_e) => respond(500, "Database error", vec![], json!({})).into_response(),
    }
}

/// List the caller's file requests (newest first).
pub async fn list_requests(
    State(st): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
) -> impl IntoResponse {
    let owner = match require_account(&caller) {
        Ok(u) => u,
        Err(r) => return r,
    };

    let rows = sqlx::query!(
        "SELECT id, token, label, message, folder_id, max_uploads, upload_count, expires_at, active, created_at \
         FROM file_requests WHERE owner_user_id = $1 ORDER BY created_at DESC LIMIT 200",
        owner
    )
    .fetch_all(&st.pg_pool)
    .await;

    match rows {
        Ok(list) => {
            let out: Vec<_> = list
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id, "token": r.token, "label": r.label, "message": r.message,
                        "folder_id": r.folder_id, "max_uploads": r.max_uploads,
                        "upload_count": r.upload_count, "expires_at": r.expires_at,
                        "active": r.active, "created_at": r.created_at
                    })
                })
                .collect();
            respond(200, "Requests", vec![], json!({ "requests": out })).into_response()
        }
        Err(_e) => respond(500, "Database error", vec![], json!({})).into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct RequestIdPayload {
    pub id: String,
}

/// The files received through one of the caller's requests.
pub async fn received_files(
    State(st): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(p): Json<RequestIdPayload>,
) -> impl IntoResponse {
    let owner = match require_account(&caller) {
        Ok(u) => u,
        Err(r) => return r,
    };

    // Verify the caller owns this request.
    let req = sqlx::query!(
        "SELECT owner_user_id FROM file_requests WHERE id = $1",
        p.id
    )
    .fetch_optional(&st.pg_pool)
    .await;
    match req {
        Ok(Some(r)) if r.owner_user_id == owner => {}
        _ => return respond(404, "Not found", vec![], json!({})).into_response(),
    }

    let files = sqlx::query_as!(
        models::File,
        "SELECT * FROM files \
         WHERE id IN (SELECT file_id FROM request_uploads WHERE request_id = $1) \
           AND deleted = false ORDER BY created_on DESC LIMIT 500",
        p.id
    )
    .fetch_all(&st.pg_pool)
    .await;

    match files {
        Ok(list) => respond(200, "Received files", vec![], json!({ "files": list })).into_response(),
        Err(_e) => respond(500, "Database error", vec![], json!({})).into_response(),
    }
}

/// Deactivate (or reactivate) a request the caller owns.
#[derive(serde::Deserialize)]
pub struct DeactivatePayload {
    pub id: String,
    #[serde(default)]
    pub active: bool,
}

pub async fn set_active(
    State(st): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(p): Json<DeactivatePayload>,
) -> impl IntoResponse {
    let owner = match require_account(&caller) {
        Ok(u) => u,
        Err(r) => return r,
    };
    let res = sqlx::query!(
        "UPDATE file_requests SET active = $1 WHERE id = $2 AND owner_user_id = $3 RETURNING id",
        p.active,
        p.id,
        owner
    )
    .fetch_optional(&st.pg_pool)
    .await;
    match res {
        Ok(Some(_)) => respond(200, "Updated", vec![], json!({ "active": p.active })).into_response(),
        Ok(None) => respond(404, "Not found", vec![], json!({})).into_response(),
        Err(_e) => respond(500, "Database error", vec![], json!({})).into_response(),
    }
}

// ===========================================================================
// Public side (the uploader, no account needed) - authorized by the token
// ===========================================================================

struct ActiveRequest {
    id: String,
    owner_user_id: String,
    folder_id: Option<String>,
    max_uploads: Option<i32>,
    upload_count: i32,
}

/// Resolve a token to an ACTIVE, unexpired, under-cap request, or an error
/// response. Shared by the public info + upload handlers.
async fn resolve_active(
    st: &crate::AppState,
    token: &str,
) -> Result<ActiveRequest, axum::response::Response> {
    let row = sqlx::query!(
        "SELECT id, owner_user_id, folder_id, max_uploads, upload_count, expires_at, active \
         FROM file_requests WHERE token = $1",
        token
    )
    .fetch_optional(&st.pg_pool)
    .await;
    let r = match row {
        Ok(Some(r)) => r,
        Ok(None) => return Err(respond(404, "Request not found", vec![], json!({})).into_response()),
        Err(_e) => return Err(respond(500, "Database error", vec![], json!({})).into_response()),
    };
    if !r.active {
        return Err(respond(410, "This request is closed.", vec![], json!({})).into_response());
    }
    if let Some(exp) = r.expires_at {
        if exp < Utc::now() {
            return Err(respond(410, "This request has expired.", vec![], json!({})).into_response());
        }
    }
    if let Some(cap) = r.max_uploads {
        if r.upload_count >= cap {
            return Err(respond(410, "This request is full.", vec![], json!({})).into_response());
        }
    }
    Ok(ActiveRequest {
        id: r.id,
        owner_user_id: r.owner_user_id,
        folder_id: r.folder_id,
        max_uploads: r.max_uploads,
        upload_count: r.upload_count,
    })
}

/// Throttle the public side of a file request.
///
/// These three endpoints are unauthenticated by design: the request token is the
/// only credential, and whoever the owner sent the link to is usually not a user
/// at all. That also means anyone holding the link can loop uploads into the
/// owner's quota, or grind `public_info` to enumerate tokens, and none of it was
/// metered. Two buckets, matching the pattern share links already use: per-IP
/// stops one source, per-token stops a distributed run at a single request.
fn request_allowed(state: &crate::AppState, ip: &str, token: &str, action: &str, per_ip: u32, per_token: u32) -> bool {
    const WINDOW: std::time::Duration = std::time::Duration::from_secs(600);
    state.rate_limiter.check(&format!("req{}:{}", action, ip), per_ip, WINDOW)
        && state.rate_limiter.check(&format!("req{}tok:{}", action, token), per_token, WINDOW)
}

fn too_many() -> axum::response::Response {
    respond(
        429,
        "Too many attempts",
        vec!["Too many requests for this link. Try again in a few minutes.".to_string()],
        json!({}),
    )
    .into_response()
}

/// Public: what is this request for (label + message), so the upload page can
/// render before anyone uploads. Reveals nothing about the owner beyond the copy
/// they wrote.
pub async fn public_info(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(st): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Path(token): Path<String>,
) -> impl IntoResponse {
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !request_allowed(&st, &ip, &token, "info", 120, 240) {
        return too_many();
    }
    let row = sqlx::query!(
        "SELECT label, message, max_uploads, upload_count, expires_at, active \
         FROM file_requests WHERE token = $1",
        token
    )
    .fetch_optional(&st.pg_pool)
    .await;
    match row {
        Ok(Some(r)) => {
            let open = r.active
                && r.expires_at.map(|e| e > Utc::now()).unwrap_or(true)
                && r.max_uploads.map(|c| r.upload_count < c).unwrap_or(true);
            respond(
                200,
                "Request",
                vec![],
                json!({ "label": r.label, "message": r.message, "open": open }),
            )
            .into_response()
        }
        Ok(None) => respond(404, "Request not found", vec![], json!({})).into_response(),
        Err(_e) => respond(500, "Database error", vec![], json!({})).into_response(),
    }
}

#[derive(serde::Deserialize)]
pub struct UploadCreatePayload {
    pub token: String,
    pub uploader_name: Option<String>,
    pub file_encrypted: bool,
    pub file_name: String,
    pub file_mime: String,
    pub file_size: i64,
    pub chunks: Vec<super::create_files::ChunkData>,
    pub sha256_checksum: String,
    pub blake3_checksum: String,
}

/// Public: begin an upload into the request owner's account. Isolated copy of the
/// create-file logic, authorized by the request token instead of a caller, and
/// always writing a sanctum (account) file owned by the request owner.
pub async fn upload_create(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(st): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(p): Json<UploadCreatePayload>,
) -> impl IntoResponse {
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    // Deliberately tighter than the read endpoints: each of these consumes the
    // owner's storage.
    if !request_allowed(&st, &ip, &p.token, "up", 20, 40) {
        return too_many();
    }
    let req = match resolve_active(&st, &p.token).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    // The upload counts against the OWNER's quota.
    if let Some(q) = crate::libs::quota::for_user(&st.pg_pool, &req.owner_user_id).await {
        if q.would_exceed(p.file_size) {
            return respond(
                413,
                "The recipient is out of storage",
                vec!["This request's owner does not have enough space for this file.".to_string()],
                json!({}),
            )
            .into_response();
        }
    }

    // The received file is a normal sanctum file owned by the request owner.
    let file = match sqlx::query_as!(
        models::File,
        "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, sha256_checksum, blake3_checksum, public_access, folder_id, owner_api_key) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, false, $9, NULL) RETURNING *",
        req.owner_user_id,
        p.file_name,
        p.file_mime,
        p.file_size,
        p.file_encrypted,
        p.chunks.len() as i64,
        p.sha256_checksum,
        p.blake3_checksum,
        req.folder_id,
    )
    .fetch_one(&st.pg_pool)
    .await
    {
        Ok(f) => f,
        Err(_e) => return respond(500, "Failed to create upload", vec![], json!({})).into_response(),
    };

    // Tie it to the request.
    if let Err(_e) = sqlx::query!(
        "INSERT INTO request_uploads (request_id, file_id, uploader_name) VALUES ($1, $2, $3)",
        req.id,
        file.id,
        p.uploader_name
    )
    .execute(&st.pg_pool)
    .await
    {
        return respond(500, "Failed to record upload", vec![], json!({})).into_response();
    }

    // Presigned PUT urls for each chunk (sanctum bucket).
    let mut chunks = Vec::new();
    let mut idx = 0i32;
    for chunk in &p.chunks {
        let chunk_id = format!("{}-{}", crate::libs::rng::uuid(), crate::libs::rng::uuid());
        let url = match st.r2.presigned_put_url("sanctum", &chunk_id).await {
            Ok(u) => u,
            Err(_e) => return respond(500, "Failed to generate presigned URL", vec![], json!({})).into_response(),
        };
        let inserted = sqlx::query_as::<_, models::Chunk>(
            "INSERT INTO chunks (id, file_id, chunk_index, size, presigned_url, file_offset, size_on_server, checksum, salt, nonce) \
             VALUES ($1, $2, $3, $4, $5, $6, 0, $7, $8, $9) RETURNING *",
        )
        .bind(chunk_id)
        .bind(file.id.clone())
        .bind(idx)
        .bind(chunk.size)
        .bind(url)
        .bind(chunk.start)
        .bind(chunk.checksum.clone())
        .bind(chunk.salt.clone())
        .bind(chunk.nonce.clone())
        .fetch_one(&st.pg_pool)
        .await;
        match inserted {
            Ok(c) => chunks.push(c),
            Err(_e) => return respond(500, "Failed to create file chunk", vec![], json!({})).into_response(),
        }
        idx += 1;
    }

    respond(200, "Upload started", vec![], json!({ "file": file, "chunks": chunks })).into_response()
}

#[derive(serde::Deserialize)]
pub struct UploadCompletePayload {
    pub token: String,
    pub chunk_id: String,
}

/// Public: mark a request-upload chunk complete. Authorized by the token, and the
/// chunk must belong to a file that was uploaded through THIS request.
pub async fn upload_mark_complete(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(st): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(p): Json<UploadCompletePayload>,
) -> impl IntoResponse {
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    // One call per chunk, so the ceiling has to clear a large multi-chunk upload
    // while still bounding a flood.
    if !request_allowed(&st, &ip, &p.token, "done", 2000, 4000) {
        return too_many();
    }
    let req = match resolve_active(&st, &p.token).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    // The chunk's file must be one uploaded through this request.
    let owner_check = sqlx::query!(
        "SELECT c.file_id FROM chunks c \
         JOIN request_uploads ru ON ru.file_id = c.file_id \
         WHERE c.id = $1 AND ru.request_id = $2",
        p.chunk_id,
        req.id
    )
    .fetch_optional(&st.pg_pool)
    .await;
    let file_id = match owner_check {
        Ok(Some(r)) => r.file_id,
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})).into_response(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    // Record the real stored size (like the main path), best effort.
    let real = st.r2.object_size("sanctum", &p.chunk_id).await.ok();
    let upd = if let Some(sz) = real {
        sqlx::query!(
            "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = $1 WHERE id = $2",
            sz,
            p.chunk_id
        )
        .execute(&st.pg_pool)
        .await
    } else {
        sqlx::query!(
            "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = size WHERE id = $1",
            p.chunk_id
        )
        .execute(&st.pg_pool)
        .await
    };
    if upd.is_err() {
        return respond(500, "Failed to update chunk status", vec![], json!({})).into_response();
    }

    // Recount completed chunks; if the file is now complete, bump upload_count.
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM chunks WHERE file_id = $1 AND uploaded = true",
        file_id
    )
    .fetch_one(&st.pg_pool)
    .await;
    if let Ok(rec) = count {
        let n = rec.count.unwrap_or(0) as i32;
        let _ = sqlx::query!("UPDATE files SET uploaded_chunks = $1 WHERE id = $2", n, file_id)
            .execute(&st.pg_pool)
            .await;
        let total: Option<i32> =
            sqlx::query_scalar("SELECT total_chunks FROM files WHERE id = $1")
                .bind(&file_id)
                .fetch_optional(&st.pg_pool)
                .await
                .ok()
                .flatten();
        if let Some(t) = total {
            if n >= t {
                // Only count the request upload once, when the file completes.
                let _ = sqlx::query!(
                    "UPDATE file_requests SET upload_count = upload_count + 1 WHERE id = $1",
                    req.id
                )
                .execute(&st.pg_pool)
                .await;
            }
        }
    }

    let _ = req.max_uploads; // (cap already enforced at create time)
    let _ = req.upload_count;
    respond(200, "Chunk complete", vec![], json!({})).into_response()
}
