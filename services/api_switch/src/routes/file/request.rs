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

/// Open a transaction holding the account row, for a write that must not land
/// for a key the account has dropped since the request was let in: a request
/// link lets anyone with it put files into the owner's drive (libs::key_guard).
async fn bound_to_key(
    st: &crate::AppState,
    caller: &Option<Caller>,
    owner: &str,
) -> Result<sqlx::Transaction<'static, sqlx::Postgres>, axum::response::Response> {
    let db_error = || respond(500, "Database error", vec![], json!({})).into_response();
    let key_index = caller.as_ref().map(|c| c.api_key.as_str()).unwrap_or_default();
    let mut tx = st.pg_pool.begin().await.map_err(|_| db_error())?;
    match crate::libs::key_guard::still_current(&mut tx, owner, key_index).await {
        Ok(true) => Ok(tx),
        Ok(false) => Err(respond(
            401,
            "Unauthorized",
            vec!["A logged-in account is required.".to_string()],
            json!({}),
        )
        .into_response()),
        Err(_) => Err(db_error()),
    }
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

    // A link puts files into the owner's account, so an account that may not
    // upload may not open one either.
    if let Some(ban) = crate::libs::bans::user_ban(&st.pg_pool, &owner).await {
        return respond(
            403,
            "Account banned",
            vec![ban.reason.clone().unwrap_or_else(|| "Your account has been banned.".to_string())],
            json!({ "banned": true, "reason": ban.reason, "until": ban.until }),
        )
        .into_response();
    }
    if crate::libs::bans::user_restricted(&st.pg_pool, &owner).await {
        return respond(
            403,
            "Account restricted",
            vec!["Your account is restricted; uploads are disabled.".to_string()],
            json!({ "restricted": true }),
        )
        .into_response();
    }

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

    let mut tx = match bound_to_key(&st, &caller, &owner).await {
        Ok(tx) => tx,
        Err(r) => return r,
    };
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
    .fetch_one(&mut *tx)
    .await;
    let row = match row {
        Ok(r) => tx.commit().await.map(|()| r),
        Err(e) => Err(e),
    };

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
    let mut tx = match bound_to_key(&st, &caller, &owner).await {
        Ok(tx) => tx,
        Err(r) => return r,
    };
    let res = sqlx::query!(
        "UPDATE file_requests SET active = $1 WHERE id = $2 AND owner_user_id = $3 RETURNING id",
        p.active,
        p.id,
        owner
    )
    .fetch_optional(&mut *tx)
    .await;
    let res = match res {
        Ok(found) => tx.commit().await.map(|()| found),
        Err(e) => Err(e),
    };
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
}

/// Whether the owner may still take files through a request link: not under an
/// active ban, not restricted, and still on a paid plan. These are the account
/// checks create_files and create_request make, applied to links that were
/// opened before the owner was banned, restricted or stopped paying. "Paid" is
/// the same as plans::is_paid: a signup promo row gives storage, not this.
async fn owner_receives(pool: &sqlx::PgPool, owner: &str) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT NOT (u.is_banned AND (u.banned_until IS NULL OR u.banned_until > NOW())) \
                AND NOT COALESCE(u.is_restricted, false) \
                AND EXISTS (SELECT 1 FROM subscriptions s \
                            WHERE s.created_by = u.id AND s.expires_on > NOW() \
                              AND s.name <> 'Promo') \
         FROM users u WHERE u.id = $1",
    )
    .bind(owner)
    .fetch_optional(pool)
    .await
    .map(|open| open.unwrap_or(false))
}

/// Resolve a token to an ACTIVE, unexpired request whose owner may still
/// receive files, or an error response. Shared by the public upload handlers.
///
/// `starting` refuses a request whose upload slots are all taken. Each upload
/// takes its slot when it starts (see upload_create), so a chunk of an upload
/// already holding one is let through even when the request is full.
async fn resolve_active(
    pool: &sqlx::PgPool,
    token: &str,
    starting: bool,
) -> Result<ActiveRequest, axum::response::Response> {
    let row = sqlx::query!(
        "SELECT id, owner_user_id, folder_id, max_uploads, upload_count, expires_at, active \
         FROM file_requests WHERE token = $1",
        token
    )
    .fetch_optional(pool)
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
        if starting && r.upload_count >= cap {
            return Err(full());
        }
    }
    // The uploader is told only that the link is closed, nothing about why.
    match owner_receives(pool, &r.owner_user_id).await {
        Ok(true) => {}
        Ok(false) => return Err(respond(410, "This request is closed.", vec![], json!({})).into_response()),
        Err(_e) => return Err(respond(500, "Database error", vec![], json!({})).into_response()),
    }
    Ok(ActiveRequest {
        id: r.id,
        owner_user_id: r.owner_user_id,
        folder_id: r.folder_id,
    })
}

fn full() -> axum::response::Response {
    respond(410, "This request is full.", vec![], json!({})).into_response()
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
    type Info = (Option<String>, Option<String>, Option<i32>, i32, Option<chrono::DateTime<Utc>>, bool, String);
    let row = sqlx::query_as::<_, Info>(
        "SELECT label, message, max_uploads, upload_count, expires_at, active, owner_user_id \
         FROM file_requests WHERE token = $1",
    )
    .bind(&token)
    .fetch_optional(&st.pg_pool)
    .await;
    match row {
        Ok(Some((label, message, max_uploads, upload_count, expires_at, active, owner))) => {
            // The same checks resolve_active makes before an upload, so the
            // page never shows a link as open that would refuse the file.
            let open = active
                && expires_at.map(|e| e > Utc::now()).unwrap_or(true)
                && max_uploads.map(|c| upload_count < c).unwrap_or(true)
                && owner_receives(&st.pg_pool, &owner).await.unwrap_or(false);
            respond(200, "Request", vec![], json!({ "label": label, "message": message, "open": open }))
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
    // The uploader has no account, so this is the anonymous upload gate.
    if let Some(ban) = crate::libs::bans::ip_ban(&st.pg_pool, &ip).await {
        return respond(
            403,
            "You are banned",
            vec![ban.reason.clone().unwrap_or_else(|| "Your access has been banned.".to_string())],
            json!({ "banned": true, "reason": ban.reason, "until": ban.until }),
        )
        .into_response();
    }
    let req = match resolve_active(&st.pg_pool, &p.token, true).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    // A file with no chunks never completes, and the orphan sweep only takes
    // files with chunks still missing, so it would hold its slot for good. The
    // upload page always sends at least one chunk, even for an empty file.
    if p.chunks.is_empty() {
        return respond(400, "Invalid upload", vec!["An upload needs at least one chunk.".to_string()], json!({}))
            .into_response();
    }

    // Same shape rules as create_files: the declared sizes are what the quota
    // check and the signed PUT lengths rely on.
    let chunk_sizes: Vec<i64> = p.chunks.iter().map(|c| c.size).collect();
    if let Err(why) = crate::libs::uploads::check_declared(p.file_size, &chunk_sizes) {
        return respond(400, "Invalid upload", vec![why.to_string()], json!({})).into_response();
    }
    if let Err(why) = crate::libs::input_validators::entry_name(&p.file_name) {
        return respond(400, "Invalid file name", vec![why], json!({})).into_response();
    }

    // The upload counts against the OWNER's quota, along with every upload
    // into that account that is started but not finished. The lock is held
    // until the file and its chunks are committed.
    let mut tx = match st.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };
    if crate::libs::quota::lock(&mut tx, &req.owner_user_id).await.is_err() {
        return respond(500, "Database error", vec![], json!({})).into_response();
    }
    if let Some(q) = crate::libs::quota::for_user(&mut tx, &req.owner_user_id).await {
        if q.would_exceed(crate::libs::uploads::stored_total(&chunk_sizes, p.file_encrypted)) {
            return respond(
                413,
                "The recipient is out of storage",
                vec!["This request's owner does not have enough space for this file.".to_string()],
                json!({}),
            )
            .into_response();
        }
    }

    // Take one of the request's upload slots now, in this transaction, so a
    // capped link cannot start more uploads than its cap before any finishes.
    // The orphan sweep gives the slot back if the upload is never finished.
    let slot = sqlx::query_scalar::<_, String>(
        "UPDATE file_requests SET upload_count = upload_count + 1 \
         WHERE id = $1 AND (max_uploads IS NULL OR upload_count < max_uploads) RETURNING id",
    )
    .bind(&req.id)
    .fetch_optional(&mut *tx)
    .await;
    match slot {
        Ok(Some(_)) => {}
        Ok(None) => return full(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
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
    .fetch_one(&mut *tx)
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
    .execute(&mut *tx)
    .await
    {
        return respond(500, "Failed to record upload", vec![], json!({})).into_response();
    }

    // Presigned PUT urls for each chunk (sanctum bucket).
    let mut chunks = Vec::new();
    let mut idx = 0i32;
    for chunk in &p.chunks {
        let chunk_id = format!("{}-{}", crate::libs::rng::uuid(), crate::libs::rng::uuid());
        let len = crate::libs::uploads::stored_len(chunk.size, p.file_encrypted);
        let url = match st.r2.presigned_put_url("sanctum", &chunk_id, len).await {
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
        .fetch_one(&mut *tx)
        .await;
        match inserted {
            Ok(c) => chunks.push(c),
            Err(_e) => return respond(500, "Failed to create file chunk", vec![], json!({})).into_response(),
        }
        idx += 1;
    }

    if tx.commit().await.is_err() {
        return respond(500, "Failed to create upload", vec![], json!({})).into_response();
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
    let req = match resolve_active(&st.pg_pool, &p.token, false).await {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    // The chunk's file must be one uploaded through this request, and not
    // given up (see upload_abandon) or put in the trash by the owner.
    let owner_check = sqlx::query_scalar::<_, String>(
        "SELECT c.file_id FROM chunks c \
         JOIN request_uploads ru ON ru.file_id = c.file_id \
         JOIN files f ON f.id = c.file_id \
         WHERE c.id = $1 AND ru.request_id = $2 AND f.deleted = false",
    )
    .bind(&p.chunk_id)
    .bind(&req.id)
    .fetch_optional(&st.pg_pool)
    .await;
    let file_id = match owner_check {
        Ok(Some(id)) => id,
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})).into_response(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    // Count the chunk only once its object is measured at the length its URL
    // was signed for, as the main path does. The declared size is never used.
    let expected = match crate::libs::uploads::chunk_expected_len(&st.pg_pool, &p.chunk_id).await {
        Ok(Some(n)) => n,
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})).into_response(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };
    let sz = match crate::libs::uploads::measure_chunk(&st.r2, "sanctum", &p.chunk_id, expected).await {
        crate::libs::uploads::Measured::Stored(sz) => sz,
        crate::libs::uploads::Measured::WrongSize => {
            return respond(400, "Chunk does not match its declared size", vec![], json!({})).into_response()
        }
        crate::libs::uploads::Measured::Unconfirmed => {
            return respond(503, "Could not confirm the chunk was stored. Try again.", vec![], json!({}))
                .into_response()
        }
    };
    let upd = sqlx::query!(
        "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = $1 WHERE id = $2",
        sz,
        p.chunk_id
    )
    .execute(&st.pg_pool)
    .await;
    if upd.is_err() {
        return respond(500, "Failed to update chunk status", vec![], json!({})).into_response();
    }

    // Recount completed chunks. The upload already holds its slot on the
    // request from upload_create, so finishing it, or marking a chunk of a
    // finished file again, does not count it a second time.
    let count = sqlx::query!(
        "SELECT COUNT(*) as count FROM chunks WHERE file_id = $1 AND uploaded = true",
        file_id
    )
    .fetch_one(&st.pg_pool)
    .await;
    if let Ok(rec) = count {
        let n = rec.count.unwrap_or(0) as i32;
        // Not on a file given up since the check above. This and the update in
        // upload_abandon both take the file row, so a file cannot be finished
        // and have its slot given back as well.
        let _ = sqlx::query("UPDATE files SET uploaded_chunks = $1 WHERE id = $2 AND deleted = false")
            .bind(n)
            .bind(&file_id)
            .execute(&st.pg_pool)
            .await;
    }

    respond(200, "Chunk complete", vec![], json!({})).into_response()
}

#[derive(serde::Deserialize)]
pub struct UploadAbandonPayload {
    pub token: String,
    pub file_id: String,
}

/// Public: give up an unfinished upload, so its slot on the request goes back
/// at once. The upload page calls this when an upload fails, so that trying
/// again works on a capped link instead of finding the link full until the
/// orphan sweep reaps the dead upload hours later.
///
/// The file row stays, in the owner's trash and untied from the request, for
/// the sweep to reap with its objects: a PUT already in flight may still land
/// after this, and the sweep waits out every PUT URL before it deletes.
/// A finished upload cannot be given up, so this never reopens a slot whose
/// file was received.
pub async fn upload_abandon(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(st): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(p): Json<UploadAbandonPayload>,
) -> impl IntoResponse {
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    // At most one per upload started, so the same ceiling as upload_create.
    if !request_allowed(&st, &ip, &p.token, "drop", 20, 40) {
        return too_many();
    }
    let not_found = || respond(404, "Upload not found", vec![], json!({})).into_response();
    let db_error = || respond(500, "Database error", vec![], json!({})).into_response();

    let mut tx = match st.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return db_error(),
    };
    // Taking the file row here is what upload_mark_complete's last update
    // waits on, so whichever of the two runs second sees the other's result.
    let given_up = sqlx::query_scalar::<_, String>(
        "UPDATE files SET deleted = true, deleted_on = NOW() \
         WHERE id = $1 AND deleted = false AND uploaded_chunks < total_chunks \
           AND id IN (SELECT ru.file_id FROM request_uploads ru \
                      JOIN file_requests r ON r.id = ru.request_id WHERE r.token = $2) \
         RETURNING id",
    )
    .bind(&p.file_id)
    .bind(&p.token)
    .fetch_optional(&mut *tx)
    .await;
    match given_up {
        Ok(Some(_)) => {}
        Ok(None) => return not_found(),
        Err(_e) => return db_error(),
    }
    if crate::libs::uploads::release_request_slots(&mut *tx, std::slice::from_ref(&p.file_id)).await.is_err() {
        return db_error();
    }
    if tx.commit().await.is_err() {
        return db_error();
    }
    respond(200, "Upload abandoned", vec![], json!({})).into_response()
}

#[cfg(test)]
mod tests {
    //! These call the handlers against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Each deletes the
    //! accounts it made.

    use super::*;
    use crate::test_support::{account, forget, state, subscribe};
    use axum::extract::ConnectInfo;
    use axum::http::{HeaderMap, StatusCode};

    /// A paying owner with one open request link. Returns (owner, token).
    async fn open_link(st: &crate::AppState) -> (String, String) {
        let (owner, _) = account(&st.pg_pool).await;
        subscribe(&st.pg_pool, &owner).await;
        let token = crate::libs::rng::uuid().replace('-', "");
        sqlx::query("INSERT INTO file_requests (token, owner_user_id) VALUES ($1, $2)")
            .bind(&token)
            .bind(&owner)
            .execute(&st.pg_pool)
            .await
            .expect("seed request");
        (owner, token)
    }

    async fn upload(st: &crate::AppState, token: &str, ip: &str) -> StatusCode {
        let mut headers = HeaderMap::new();
        headers.insert("x-client-ip", ip.parse().unwrap());
        let payload: UploadCreatePayload = serde_json::from_value(json!({
            "token": token,
            "uploader_name": "someone",
            "file_encrypted": false,
            "file_name": "notes.txt",
            "file_mime": "text/plain",
            "file_size": 5,
            "chunks": [{ "start": 0, "end": 5, "size": 5, "checksum": "c" }],
            "sha256_checksum": "s",
            "blake3_checksum": "b"
        }))
        .unwrap();
        upload_create(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), headers, Json(payload))
            .await
            .into_response()
            .status()
    }

    async fn files_of(st: &crate::AppState, owner: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE user_id = $1")
            .bind(owner)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn set(st: &crate::AppState, sql: &str, owner: &str) {
        sqlx::query(sql).bind(owner).execute(&st.pg_pool).await.expect(sql);
    }

    fn some_ip() -> String {
        format!("2001:db8::{}", &crate::libs::rng::uuid()[..4])
    }

    #[tokio::test]
    async fn uploads_through_a_link_share_the_owner_plan_before_any_is_stored() {
        // Each create was checked against stored bytes only, so a link holder
        // could start any number of uploads before the first one landed.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(&st, "UPDATE users SET default_storage_bytes = 8 WHERE id = $1", &owner).await;
        let first = upload(&st, &token, &some_ip()).await;
        let second = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(first, StatusCode::OK);
        assert_eq!(second, StatusCode::PAYLOAD_TOO_LARGE);
        assert_eq!(made, 1);
    }

    #[tokio::test]
    async fn an_open_link_of_a_paying_owner_still_takes_uploads() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let status = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(made, 1);
    }

    #[tokio::test]
    async fn a_banned_owner_link_takes_no_uploads() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NULL WHERE id = $1", &owner).await;
        let status = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_lapsed_temporary_ban_reopens_the_link() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(
            &st,
            "UPDATE users SET is_banned = TRUE, banned_until = NOW() - INTERVAL '1 hour' WHERE id = $1",
            &owner,
        )
        .await;
        let status = upload(&st, &token, &some_ip()).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_restricted_owner_link_takes_no_uploads() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(&st, "UPDATE users SET is_restricted = TRUE WHERE id = $1", &owner).await;
        let status = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_link_stops_taking_uploads_when_the_plan_lapses() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(&st, "UPDATE subscriptions SET expires_on = NOW() - INTERVAL '1 day' WHERE created_by = $1", &owner).await;
        let status = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_link_of_an_owner_left_with_only_a_signup_promo_is_closed() {
        // A link opened while the owner paid, or while a signup promo still
        // counted as paid, kept taking files on the promo row alone.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        set(&st, "UPDATE subscriptions SET name = 'Promo' WHERE created_by = $1", &owner).await;
        let status = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        let page = public_info(
            ConnectInfo("127.0.0.1:1".parse().unwrap()),
            State(st.clone()),
            HeaderMap::new(),
            Path(token.clone()),
        )
        .await
        .into_response();
        let body = axum::body::to_bytes(page.into_body(), usize::MAX).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::GONE);
        assert_eq!(made, 0);
        assert_eq!(v.pointer("/data/open").cloned(), Some(json!(false)));
    }

    #[tokio::test]
    async fn an_ip_banned_uploader_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let ip = some_ip();
        let ban = crate::libs::rng::uuid();
        sqlx::query("INSERT INTO ip_bans (id, ip, reason) VALUES ($1, $2, 'abuse')")
            .bind(&ban)
            .bind(&ip)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let status = upload(&st, &token, &ip).await;
        let made = files_of(&st, &owner).await;
        sqlx::query("DELETE FROM ip_bans WHERE id = $1").bind(&ban).execute(&st.pg_pool).await.unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn the_upload_page_shows_a_restricted_owner_link_as_closed() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let open = |st: crate::AppState, token: String| async move {
            let resp = public_info(
                ConnectInfo("127.0.0.1:1".parse().unwrap()),
                State(st),
                HeaderMap::new(),
                Path(token),
            )
            .await
            .into_response();
            let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
            let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
            v.pointer("/data/open").cloned()
        };
        let before = open(st.clone(), token.clone()).await;
        set(&st, "UPDATE users SET is_restricted = TRUE WHERE id = $1", &owner).await;
        let after = open(st.clone(), token.clone()).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(before, Some(json!(true)));
        assert_eq!(after, Some(json!(false)));
    }

    #[tokio::test]
    async fn a_restricted_account_cannot_open_a_request_link() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, key_index) = account(&st.pg_pool).await;
        subscribe(&st.pg_pool, &owner).await;
        set(&st, "UPDATE users SET is_restricted = TRUE WHERE id = $1", &owner).await;
        let caller = Some(Caller { api_key: key_index, user_id: Some(owner.clone()), email_verified: true });
        let payload: CreateRequestPayload = serde_json::from_value(json!({})).unwrap();
        let status =
            create_request(State(st.clone()), Extension(caller), Json(payload)).await.into_response().status();
        let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM file_requests WHERE owner_user_id = $1")
            .bind(&owner)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(links, 0);
    }

    #[tokio::test]
    async fn a_banned_account_cannot_open_a_request_link() {
        // resolve_identity already turns a banned key away before this handler
        // runs; this holds the handler's own refusal for any route that does not.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, key_index) = account(&st.pg_pool).await;
        subscribe(&st.pg_pool, &owner).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NULL WHERE id = $1", &owner).await;
        let caller = Some(Caller { api_key: key_index, user_id: Some(owner.clone()), email_verified: true });
        let payload: CreateRequestPayload = serde_json::from_value(json!({})).unwrap();
        let status =
            create_request(State(st.clone()), Extension(caller), Json(payload)).await.into_response().status();
        let links: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM file_requests WHERE owner_user_id = $1")
            .bind(&owner)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(links, 0);
    }

    async fn upload_body(st: &crate::AppState, token: &str, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let mut headers = HeaderMap::new();
        headers.insert("x-client-ip", some_ip().parse().unwrap());
        let mut body = body;
        body["token"] = json!(token);
        let payload: UploadCreatePayload = serde_json::from_value(body).unwrap();
        let resp = upload_create(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), headers, Json(payload))
            .await
            .into_response();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
    }

    #[tokio::test]
    async fn a_request_link_refuses_a_negative_chunk() {
        // Through a link, an outsider could drive the OWNER's counted usage
        // negative (or to +2^62, locking the owner out of their own uploads).
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let (status, _) = upload_body(
            &st,
            &token,
            json!({
                "uploader_name": null,
                "file_encrypted": false,
                "file_name": "x",
                "file_mime": "application/octet-stream",
                "file_size": 0,
                "chunks": [{ "start": 0, "end": 0, "size": -4611686018427387904i64, "checksum": "c" }],
                "sha256_checksum": "s",
                "blake3_checksum": ""
            }),
        )
        .await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_request_chunk_that_was_never_stored_is_not_marked() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let (created, v) = upload_body(
            &st,
            &token,
            json!({
                "uploader_name": null,
                "file_encrypted": false,
                "file_name": "x",
                "file_mime": "application/octet-stream",
                "file_size": 5,
                "chunks": [{ "start": 0, "end": 5, "size": 5, "checksum": "c" }],
                "sha256_checksum": "s",
                "blake3_checksum": ""
            }),
        )
        .await;
        let chunk_id = v.pointer("/data/chunks/0/id").and_then(|c| c.as_str()).unwrap_or_default().to_string();
        let mut headers = HeaderMap::new();
        headers.insert("x-client-ip", some_ip().parse().unwrap());
        let payload: UploadCompletePayload =
            serde_json::from_value(json!({ "token": token, "chunk_id": chunk_id })).unwrap();
        let status = upload_mark_complete(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), headers, Json(payload))
            .await
            .into_response()
            .status();
        let (uploaded, on_server): (bool, i64) =
            sqlx::query_as("SELECT uploaded, size_on_server FROM chunks WHERE id = $1")
                .bind(&chunk_id)
                .fetch_one(&st.pg_pool)
                .await
                .unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!(created, StatusCode::OK);
        assert_ne!(status, StatusCode::OK);
        assert!(!uploaded);
        assert_eq!(on_server, 0);
    }

    #[tokio::test]
    async fn a_request_upload_named_as_a_path_is_refused() {
        // The owner zips what came in from the dashboard; a name like this
        // would extract outside the folder they chose.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let mut accepted = vec![];
        for name in ["../../../Startup/update.bat", "..", "a\\b", "x\u{1b}[2J"] {
            let (status, _) = upload_body(
                &st,
                &token,
                json!({
                    "uploader_name": null,
                    "file_encrypted": false,
                    "file_name": name,
                    "file_mime": "application/octet-stream",
                    "file_size": 5,
                    "chunks": [{ "start": 0, "end": 5, "size": 5, "checksum": "c" }],
                    "sha256_checksum": "s",
                    "blake3_checksum": ""
                }),
            )
            .await;
            if status != StatusCode::BAD_REQUEST {
                accepted.push((name, status));
            }
        }
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert!(accepted.is_empty(), "accepted: {accepted:?}");
        assert_eq!(made, 0);
    }

    /// A paying owner with one open request link capped at `cap` uploads.
    async fn capped_link(st: &crate::AppState, cap: i32) -> (String, String) {
        let (owner, token) = open_link(st).await;
        sqlx::query("UPDATE file_requests SET max_uploads = $1 WHERE token = $2")
            .bind(cap)
            .bind(&token)
            .execute(&st.pg_pool)
            .await
            .expect("cap request");
        (owner, token)
    }

    #[tokio::test]
    async fn a_capped_link_takes_no_more_uploads_than_its_cap_while_they_are_in_flight() {
        // The cap was compared with a count that only moved when an upload
        // finished, so a link holder could start any number of uploads first.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let first = upload(&st, &token, &some_ip()).await;
        let second = upload(&st, &token, &some_ip()).await;
        let third = upload(&st, &token, &some_ip()).await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!((first, second, third), (StatusCode::OK, StatusCode::GONE, StatusCode::GONE));
        assert_eq!(made, 1);
    }

    #[tokio::test]
    async fn a_request_upload_with_no_chunks_is_refused() {
        // A file with no chunks can never complete, and the orphan sweep only
        // takes files with chunks still missing, so it stayed for good.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = open_link(&st).await;
        let (status, _) = upload_body(
            &st,
            &token,
            json!({
                "uploader_name": null,
                "file_encrypted": false,
                "file_name": "x",
                "file_mime": "application/octet-stream",
                "file_size": 0,
                "chunks": [],
                "sha256_checksum": "s",
                "blake3_checksum": ""
            }),
        )
        .await;
        let made = files_of(&st, &owner).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    /// Start a one-chunk, 5-byte upload through `token`.
    /// Returns (status, file id, chunk id); the ids are empty when refused.
    async fn start(st: &crate::AppState, token: &str) -> (StatusCode, String, String) {
        let (status, v) = upload_body(
            st,
            token,
            json!({
                "uploader_name": null,
                "file_encrypted": false,
                "file_name": "x",
                "file_mime": "application/octet-stream",
                "file_size": 5,
                "chunks": [{ "start": 0, "end": 5, "size": 5, "checksum": "c" }],
                "sha256_checksum": "s",
                "blake3_checksum": ""
            }),
        )
        .await;
        let text = |ptr: &str| v.pointer(ptr).and_then(|c| c.as_str()).unwrap_or_default().to_string();
        (status, text("/data/file/id"), text("/data/chunks/0/id"))
    }

    async fn mark(st: &crate::AppState, token: &str, chunk_id: &str) -> StatusCode {
        let mut headers = HeaderMap::new();
        headers.insert("x-client-ip", some_ip().parse().unwrap());
        let payload: UploadCompletePayload =
            serde_json::from_value(json!({ "token": token, "chunk_id": chunk_id })).unwrap();
        upload_mark_complete(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), headers, Json(payload))
            .await
            .into_response()
            .status()
    }

    async fn abandon(st: &crate::AppState, token: &str, file_id: &str) -> StatusCode {
        let mut headers = HeaderMap::new();
        headers.insert("x-client-ip", some_ip().parse().unwrap());
        let payload: UploadAbandonPayload =
            serde_json::from_value(json!({ "token": token, "file_id": file_id })).unwrap();
        upload_abandon(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), headers, Json(payload))
            .await
            .into_response()
            .status()
    }

    async fn used(st: &crate::AppState, token: &str) -> i32 {
        sqlx::query_scalar("SELECT upload_count FROM file_requests WHERE token = $1")
            .bind(token)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    /// (uploaded_chunks, total_chunks, deleted) of a file.
    async fn progress(st: &crate::AppState, file_id: &str) -> (i32, i32, bool) {
        sqlx::query_as("SELECT uploaded_chunks::int, total_chunks::int, deleted FROM files WHERE id = $1")
            .bind(file_id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    /// State whose storage reports every chunk stored at 5 bytes, so mark-
    /// complete can succeed.
    async fn state_with_stored_chunks() -> Option<crate::AppState> {
        let mut st = state().await?;
        st.r2 = crate::test_support::storage_holding(5);
        Some(st)
    }

    #[tokio::test]
    async fn the_last_slot_of_a_capped_link_can_still_finish_its_upload() {
        // The upload holding the last slot fills the cap, and its own chunks
        // must still be accepted after that.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (created, file_id, chunk_id) = start(&st, &token).await;
        let marked = mark(&st, &token, &chunk_id).await;
        let (uploaded, total, _) = progress(&st, &file_id).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!((created, marked), (StatusCode::OK, StatusCode::OK));
        assert_eq!(uploaded, total);
    }

    #[tokio::test]
    async fn marking_a_finished_upload_again_does_not_fill_the_cap() {
        // Each mark-complete call on the last chunk of a finished file counted
        // it again, so one link holder could fill the cap and turn away the
        // others the owner sent the link to.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 2).await;
        let (created, _, chunk_id) = start(&st, &token).await;
        let mut marks = vec![];
        for _ in 0..3 {
            marks.push(mark(&st, &token, &chunk_id).await);
        }
        let after_marks = used(&st, &token).await;
        let (second, _, _) = start(&st, &token).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(created, StatusCode::OK);
        assert_eq!(marks, vec![StatusCode::OK; 3]);
        assert_eq!(after_marks, 1);
        assert_eq!(second, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_failed_upload_given_up_lets_the_retry_through_a_capped_link() {
        // The page starts a new upload when someone tries again. On a link
        // capped at one, the dead upload held the only slot until the orphan
        // sweep reaped it hours later.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (first, failed, _) = start(&st, &token).await;
        let gave_up = abandon(&st, &token, &failed).await;
        let (retry, _, _) = start(&st, &token).await;
        let count = used(&st, &token).await;
        let (_, _, trashed) = progress(&st, &failed).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!((first, gave_up, retry), (StatusCode::OK, StatusCode::OK, StatusCode::OK));
        assert_eq!(count, 1);
        assert!(trashed);
    }

    #[tokio::test]
    async fn a_finished_upload_cannot_be_given_up() {
        // Otherwise a link holder could finish a file, give it up, and send
        // another, past any cap.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (_, file_id, chunk_id) = start(&st, &token).await;
        let marked = mark(&st, &token, &chunk_id).await;
        let gave_up = abandon(&st, &token, &file_id).await;
        let (next, _, _) = start(&st, &token).await;
        let (_, _, trashed) = progress(&st, &file_id).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(marked, StatusCode::OK);
        assert_eq!(gave_up, StatusCode::NOT_FOUND);
        assert_eq!(next, StatusCode::GONE);
        assert!(!trashed);
    }

    #[tokio::test]
    async fn an_upload_cannot_be_given_up_through_another_link() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (other_owner, other_token) = open_link(&st).await;
        let (_, file_id, _) = start(&st, &token).await;
        let gave_up = abandon(&st, &other_token, &file_id).await;
        let count = used(&st, &token).await;
        let (_, _, trashed) = progress(&st, &file_id).await;
        forget(&st.pg_pool, &owner).await;
        forget(&st.pg_pool, &other_owner).await;
        assert_eq!(gave_up, StatusCode::NOT_FOUND);
        assert_eq!(count, 1);
        assert!(!trashed);
    }

    #[tokio::test]
    async fn an_upload_given_up_cannot_be_finished() {
        // Its slot went back, so finishing it would put one file past the cap.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (_, file_id, chunk_id) = start(&st, &token).await;
        let gave_up = abandon(&st, &token, &file_id).await;
        let marked = mark(&st, &token, &chunk_id).await;
        let (uploaded, _, _) = progress(&st, &file_id).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(gave_up, StatusCode::OK);
        assert_eq!(marked, StatusCode::NOT_FOUND);
        assert_eq!(uploaded, 0);
    }

    /// Wait until a statement matching `like` is blocked on a lock.
    async fn blocked(st: &crate::AppState, like: &str) {
        for _ in 0..200 {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM pg_stat_activity \
                 WHERE datname = current_database() AND wait_event_type = 'Lock' AND query LIKE $1",
            )
            .bind(like)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
            if waiting > 0 {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        }
        panic!("nothing blocked on {like}");
    }

    #[tokio::test]
    async fn an_upload_given_up_while_its_last_chunk_is_marked_does_not_finish() {
        // Mark-complete checks the file before it measures the chunk, and only
        // records the chunk count at the end. A give-up landing in between
        // returned the slot, and the file still finished: one file past the cap.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (_, file_id, chunk_id) = start(&st, &token).await;

        // Hold the file row so both calls queue on it in a known order.
        let mut hold = st.pg_pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM files WHERE id = $1 FOR UPDATE").bind(&file_id).execute(&mut *hold).await.unwrap();
        let give_up = tokio::spawn({
            let (st, token, file_id) = (st.clone(), token.clone(), file_id.clone());
            async move { abandon(&st, &token, &file_id).await }
        });
        blocked(&st, "UPDATE files SET deleted = true%").await;
        let finish = tokio::spawn({
            let (st, token, chunk_id) = (st.clone(), token.clone(), chunk_id.clone());
            async move { mark(&st, &token, &chunk_id).await }
        });
        blocked(&st, "UPDATE files SET uploaded_chunks%").await;
        hold.commit().await.unwrap();
        let gave_up = give_up.await.unwrap();
        finish.await.unwrap();

        let (uploaded, total, _) = progress(&st, &file_id).await;
        let count = used(&st, &token).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(gave_up, StatusCode::OK);
        assert_eq!(count, 0);
        assert!(uploaded < total, "the given-up file finished with its slot returned");
    }

    fn owner_caller(owner: &str) -> Option<Caller> {
        Some(Caller { api_key: String::new(), user_id: Some(owner.to_string()), email_verified: true })
    }

    #[tokio::test]
    async fn deleting_an_unfinished_received_file_gives_its_slot_back() {
        // The owner sees a file still arriving in their received list and may
        // delete it. Its slot stayed taken for good, closing a capped link.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (_, file_id, _) = start(&st, &token).await;
        let payload: super::super::permanent_delete_files::DeletePayload =
            serde_json::from_value(json!({ "file_id": file_id })).unwrap();
        let deleted = super::super::permanent_delete_files::handle(State(st.clone()), Extension(owner_caller(&owner)), Json(payload))
            .await
            .into_response()
            .status();
        let (next, _, _) = start(&st, &token).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!(deleted, StatusCode::OK);
        assert_eq!(next, StatusCode::OK);
    }

    #[tokio::test]
    async fn deleting_a_received_file_does_not_reopen_its_slot() {
        // The cap counts files received, not files kept.
        let Some(st) = state_with_stored_chunks().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let (_, file_id, chunk_id) = start(&st, &token).await;
        let marked = mark(&st, &token, &chunk_id).await;
        let payload: super::super::permanent_delete_files::DeletePayload =
            serde_json::from_value(json!({ "file_id": file_id })).unwrap();
        let deleted = super::super::permanent_delete_files::handle(State(st.clone()), Extension(owner_caller(&owner)), Json(payload))
            .await
            .into_response()
            .status();
        let (next, _, _) = start(&st, &token).await;
        forget(&st.pg_pool, &owner).await;
        assert_eq!((marked, deleted), (StatusCode::OK, StatusCode::OK));
        assert_eq!(next, StatusCode::GONE);
    }

    #[tokio::test]
    async fn deleting_the_folder_of_an_unfinished_received_file_gives_its_slot_back() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (owner, token) = capped_link(&st, 1).await;
        let folder: String = sqlx::query_scalar("INSERT INTO folders (name, user_id) VALUES ('inbox', $1) RETURNING id")
            .bind(&owner)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        sqlx::query("UPDATE file_requests SET folder_id = $1 WHERE token = $2")
            .bind(&folder)
            .bind(&token)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let (first, _, _) = start(&st, &token).await;
        let payload: crate::routes::folder::permanent_delete_folders::DeletePayload =
            serde_json::from_value(json!({ "folder_id": folder })).unwrap();
        let deleted = crate::routes::folder::permanent_delete_folders::handle(
            State(st.clone()),
            Extension(owner_caller(&owner)),
            Json(payload),
        )
        .await
        .into_response()
        .status();
        // The link's folder is gone, so the next upload lands at the top.
        sqlx::query("UPDATE file_requests SET folder_id = NULL WHERE token = $1")
            .bind(&token)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let (next, _, _) = start(&st, &token).await;
        sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(&owner).execute(&st.pg_pool).await.unwrap();
        forget(&st.pg_pool, &owner).await;
        assert_eq!((first, deleted), (StatusCode::OK, StatusCode::OK));
        assert_eq!(next, StatusCode::OK);
    }
}
