use axum::extract::{Path, State};
use axum::{Json, Extension};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;
use crate::middlewares::resolve_identity::Caller;
use rand::Rng; // Make sure rand is available or use uuid
use sha2::{Digest, Sha256};
use chrono::{Duration, Utc};

#[derive(Deserialize)]
pub struct ToggleSharePayload {
    pub file_id: Option<String>,
    pub folder_id: Option<String>,
    pub share_type: String, // 'off', 'public', 'once'
    // Optional hardening. Fields are only applied when present, so toggling the
    // share type does not silently wipe an existing expiry/password.
    // expires_in_days: Some(0) = never, Some(n) = n days, None = leave unchanged.
    pub expires_in_days: Option<i64>,
    // password: Some(non-empty) = set, None = leave unchanged.
    pub password: Option<String>,
    // clear_password: Some(true) = remove existing password.
    pub clear_password: Option<bool>,
}

/// Hash a share-link password for storage.
///
/// Argon2id, same parameters as account passwords. A link password is often the
/// only gate on a share whose contents were never client-side encrypted, so it
/// gets treated as a real credential: per-link salt (so one rainbow table can't
/// cover every link) and a work factor that makes offline cracking expensive.
fn hash_share_pw(pw: &str) -> Option<String> {
    crate::libs::argon2::hash(pw.to_string()).ok()
}

/// Check a supplied share password against the stored hash.
///
/// Returns false when nothing was supplied. Handles both formats:
///   - `$argon2id$...` PHC strings, verified by the argon2 crate (constant time)
///   - legacy 64-char SHA-256 hex from before this was salted, compared in
///     constant time so the comparison itself leaks nothing
///
/// Callers must rate-limit before reaching here: Argon2id is deliberately
/// expensive (~19 MB, ~50 ms per call), which is a denial-of-service lever on
/// an unauthenticated endpoint if left unmetered.
fn verify_share_pw(supplied: Option<&str>, stored: &str) -> bool {
    let Some(pw) = supplied.map(str::trim).filter(|p| !p.is_empty()) else {
        return false;
    };

    if stored.starts_with("$argon2") {
        return crate::libs::argon2::verify(&pw.to_string(), stored.to_string());
    }

    // Legacy unsalted SHA-256. Still accepted so existing links keep working;
    // upgraded to Argon2id on the next successful use (see upgrade_share_pw).
    let computed = hex::encode(Sha256::digest(pw.as_bytes()));
    crate::middlewares::ct_eq(computed.as_bytes(), stored.as_bytes())
}

/// True when a stored hash is in the old unsalted format and should be rewritten.
fn is_legacy_share_pw(stored: &str) -> bool {
    !stored.starts_with("$argon2")
}

/// Throttle password attempts on a public share link.
///
/// Applied only once we know the link actually has a password, so ordinary
/// downloads are never throttled. Two buckets: per-IP stops one attacker, and
/// per-token stops a distributed attack converging on a single link. This is
/// also what keeps Argon2id from becoming a memory-exhaustion lever, so it must
/// run before any verification work.
fn share_pw_allowed(state: &crate::AppState, ip: &str, token: &str) -> bool {
    const WINDOW: std::time::Duration = std::time::Duration::from_secs(300);
    state.rate_limiter.check(&format!("sharepw:{}", ip), 30, WINDOW)
        && state.rate_limiter.check(&format!("sharepwtok:{}", token), 20, WINDOW)
}

/// Rewrite a legacy SHA-256 link password as Argon2id after a successful check.
/// Best-effort: on failure the old hash stays and the next use retries.
async fn upgrade_share_pw(pool: &sqlx::PgPool, table: &str, id: &str, pw: &str) {
    let Some(new_hash) = hash_share_pw(pw.trim()) else { return };
    let sql = if table == "files" {
        "UPDATE files SET share_password_hash = $1 WHERE id = $2"
    } else {
        "UPDATE folders SET share_password_hash = $1 WHERE id = $2"
    };
    let _ = sqlx::query(sql).bind(new_hash).bind(id).execute(pool).await;
}

#[derive(Deserialize)]
pub struct RegeneratePayload {
    pub file_id: Option<String>,
    pub folder_id: Option<String>,
}

fn generate_token() -> String {
    // Generate a secure enough random string
    let random_bytes: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    hex::encode(random_bytes)
}

pub async fn toggle_share(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<ToggleSharePayload>,
) -> impl IntoResponse {
    let new_token = generate_token();
    // Sharing is managed by the owning registered user; identity from the token.
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

    // Compute conditional updates: (should_update, value). When should_update is
    // false the existing column value is preserved (CASE WHEN in the UPDATE).
    let (exp_update, exp_value) = match payload.expires_in_days {
        Some(d) if d > 0 => (true, Some(Utc::now() + Duration::days(d))),
        Some(_) => (true, None), // 0 or negative = never
        None => (false, None),
    };
    let (pw_update, pw_value) = match (payload.password.as_ref(), payload.clear_password) {
        (Some(p), _) if !p.trim().is_empty() => match hash_share_pw(p.trim()) {
            Some(h) => (true, Some(h)),
            // Never fall through to "no password" on a hashing failure: that
            // would silently publish the share unprotected.
            None => return respond(500, "Could not set the link password", vec![], json!({})),
        },
        (_, Some(true)) => (true, None),
        _ => (false, None),
    };

    if let Some(file_id) = payload.file_id {
        let mut tx = match axum_state.pg_pool.begin().await {
            Ok(tx) => tx,
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        };

        // Fetch current file to check ownership and current token
        let current_file = sqlx::query!(
            "SELECT share_token FROM files WHERE id = $1 AND user_id = $2",
            file_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await;

        match current_file {
            Ok(Some(record)) => {
                let token_to_set = if record.share_token.is_none() && payload.share_type != "off" {
                    Some(new_token)
                } else {
                    record.share_token // Keep existing
                };

                let result = sqlx::query!(
                    "UPDATE files SET share_type = $1, share_token = $2, \
                     share_expires_at = CASE WHEN $3 THEN $4::timestamptz ELSE share_expires_at END, \
                     share_password_hash = CASE WHEN $5 THEN $6::text ELSE share_password_hash END \
                     WHERE id = $7 AND user_id = $8 \
                     RETURNING share_token, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash",
                    payload.share_type,
                    token_to_set,
                    exp_update,
                    exp_value,
                    pw_update,
                    pw_value,
                    file_id,
                    user_id
                )
                .fetch_one(&mut *tx)
                .await;

                match result {
                    Ok(r) => {
                        let _ = tx.commit().await;
                        respond(200, "Share settings updated", vec![], json!({
                            "share_token": r.share_token,
                            "share_type": r.share_type,
                            "link_downloads": r.link_downloads,
                            "link_max_downloads": r.link_max_downloads,
                            "expires_at": r.share_expires_at.map(|t| t.to_rfc3339()),
                            "password_protected": r.share_password_hash.is_some()
                        }))
                    },
                    Err(_e) => {
                         let _ = tx.rollback().await;
                         respond(500, "Failed to update share settings", vec![], json!({}))
                    }
                }
            },
            Ok(None) => respond(404, "File not found", vec![], json!({})),
            Err(_e) => respond(500, "Database error", vec![], json!({})),
        }

    } else if let Some(folder_id) = payload.folder_id {
         let mut tx = match axum_state.pg_pool.begin().await {
            Ok(tx) => tx,
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        };

        let current_folder = sqlx::query!(
            "SELECT share_token FROM folders WHERE id = $1 AND user_id = $2",
            folder_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await;
        
        match current_folder {
             Ok(Some(record)) => {
                let token_to_set = if record.share_token.is_none() && payload.share_type != "off" {
                    Some(generate_token())
                } else {
                    record.share_token
                };

                let result = sqlx::query!(
                    "UPDATE folders SET share_type = $1, share_token = $2, \
                     share_expires_at = CASE WHEN $3 THEN $4::timestamptz ELSE share_expires_at END, \
                     share_password_hash = CASE WHEN $5 THEN $6::text ELSE share_password_hash END \
                     WHERE id = $7 AND user_id = $8 \
                     RETURNING share_token, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash",
                    payload.share_type,
                    token_to_set,
                    exp_update,
                    exp_value,
                    pw_update,
                    pw_value,
                    folder_id,
                    user_id
                )
                .fetch_one(&mut *tx)
                .await;

                  match result {
                    Ok(r) => {
                        let _ = tx.commit().await;
                        respond(200, "Share settings updated", vec![], json!({
                            "share_token": r.share_token,
                            "share_type": r.share_type,
                            "link_downloads": r.link_downloads,
                            "link_max_downloads": r.link_max_downloads,
                            "expires_at": r.share_expires_at.map(|t| t.to_rfc3339()),
                            "password_protected": r.share_password_hash.is_some()
                        }))
                    },
                    Err(_e) => {
                         let _ = tx.rollback().await;
                         respond(500, "Failed to update share settings", vec![], json!({}))
                    }
                }
             },
             Ok(None) => respond(404, "Folder not found", vec![], json!({})),
             Err(_e) => respond(500, "Database error", vec![], json!({})),
        }

    } else {
        respond(400, "Missing file_id or folder_id", vec![], json!({}))
    }
}

pub async fn regenerate_token(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<RegeneratePayload>,
) -> impl IntoResponse {
    let new_token = generate_token();
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
    
    if let Some(file_id) = payload.file_id {
        let result = sqlx::query!(
            "UPDATE files SET share_token = $1, link_downloads = 0 WHERE id = $2 AND user_id = $3 RETURNING share_token",
            new_token,
            file_id,
            user_id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;

        match result {
            Ok(Some(r)) => respond(200, "Token regenerated", vec![], json!({ "share_token": r.share_token })),
            Ok(None) => respond(404, "File not found", vec![], json!({})),
            Err(_e) => respond(500, "Database error", vec![], json!({})),
        }
    } else if let Some(folder_id) = payload.folder_id {
         let result = sqlx::query!(
            "UPDATE folders SET share_token = $1, link_downloads = 0 WHERE id = $2 AND user_id = $3 RETURNING share_token",
            new_token,
            folder_id,
            user_id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;

        match result {
            Ok(Some(r)) => respond(200, "Token regenerated", vec![], json!({ "share_token": r.share_token })),
            Ok(None) => respond(404, "Folder not found", vec![], json!({})),
            Err(_e) => respond(500, "Database error", vec![], json!({})),
        }
    } else {
        respond(400, "Missing file_id or folder_id", vec![], json!({}))
    }
}

pub async fn get_share_info(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
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

    // Check files
    let file = sqlx::query!(
        "SELECT share_token, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM files WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = file {
         return respond(200, "Share info retrieved", vec![], json!({
            "type": "file",
            "share_token": r.share_token,
            "share_type": r.share_type,
            "link_downloads": r.link_downloads,
            "link_max_downloads": r.link_max_downloads,
            "expires_at": r.share_expires_at.map(|t| t.to_rfc3339()),
            "password_protected": r.share_password_hash.is_some()
        }));
    }

    // Check folders
    let folder = sqlx::query!(
        "SELECT share_token, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = folder {
         return respond(200, "Share info retrieved", vec![], json!({
            "type": "folder",
            "share_token": r.share_token,
            "share_type": r.share_type,
            "link_downloads": r.link_downloads,
            "link_max_downloads": r.link_max_downloads,
            "expires_at": r.share_expires_at.map(|t| t.to_rfc3339()),
            "password_protected": r.share_password_hash.is_some()
        }));
    }

    respond(404, "Item not found", vec![], json!({}))
}

#[derive(Deserialize)]
pub struct PublicDownloadPayload {
    pub token: String,
    pub password: Option<String>,
}

// PUBLIC ENDPOINTS (No User Auth)

#[derive(serde::Serialize)]
pub struct PublicChunkResponse {
    pub id: String,
    pub index: i32,
    pub size: i64,
    pub presigned_url: String,
    pub file_offset: i64,
    pub checksum: String,
    pub nonce: Option<String>,
    pub salt: Option<String>,
}

pub async fn public_get_info(
    State(axum_state): State<crate::AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    // Check files
    let file = sqlx::query!(
        "SELECT id, name, size, mime, share_type, link_downloads, link_max_downloads, encrypted, share_expires_at, share_password_hash FROM files WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file {
        Ok(Some(r)) => {
            if let Some(exp) = r.share_expires_at {
                if exp < Utc::now() {
                    return respond(410, "This link has expired.", vec!["Link expired".to_string()], json!({}));
                }
            }
            if r.share_type == Some("once".to_string()) {
                let downloads = r.link_downloads.unwrap_or(0);
                let max = r.link_max_downloads.unwrap_or(1);
                if downloads >= max {
                    return respond(410, "This safe-link has expired.", vec!["Link limit reached".to_string()], json!({}));
                }
            }

            // A password-protected link discloses nothing before the password is
            // supplied. Filenames and sizes are frequently the sensitive part of
            // a share, so reporting `password_required` alongside the metadata
            // (as this used to) defeated the point of setting a password.
            if r.share_password_hash.is_some() {
                return respond(200, "Password required", vec![], json!({
                    "type": "file",
                    "password_required": true
                }));
            }

            return respond(200, "File found", vec![], json!({
                "type": "file",
                "id": r.id,
                "name": r.name,
                "size": r.size,
                "mime": r.mime,
                "encrypted": r.encrypted, // Expose encrypted status
                "password_required": false,
                "expires_at": r.share_expires_at.map(|t| t.to_rfc3339())
            }));
        },
        Ok(None) => {},
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    }

    // Check folders
    let folder = sqlx::query!(
        "SELECT id, name, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder {
        Ok(Some(r)) => {
            if let Some(exp) = r.share_expires_at {
                if exp < Utc::now() {
                    return respond(410, "This link has expired.", vec!["Link expired".to_string()], json!({}));
                }
            }
            if r.share_type == Some("once".to_string()) {
                let downloads = r.link_downloads.unwrap_or(0);
                let max = r.link_max_downloads.unwrap_or(1);
                if downloads >= max {
                    return respond(410, "This safe-link has expired.", vec!["Link limit reached".to_string()], json!({}));
                }
            }

            // As above: a protected folder listing is withheld entirely until the
            // password is verified. This was the more serious of the two leaks,
            // since it exposed every filename in the folder.
            if r.share_password_hash.is_some() {
                return respond(200, "Password required", vec![], json!({
                    "type": "folder",
                    "password_required": true
                }));
            }

            // Fetch files in folder to display
            let files = sqlx::query!(
                "SELECT id, name, size, mime, encrypted FROM files WHERE folder_id = $1 AND deleted = false",
                r.id
            )
            .fetch_all(&axum_state.pg_pool)
            .await;

            let files_data = match files {
                Ok(list) => list.into_iter().map(|f| json!({
                    "id": f.id,
                    "name": f.name,
                    "size": f.size,
                    "mime": f.mime,
                    "encrypted": f.encrypted
                })).collect::<Vec<_>>(),
                Err(_) => vec![] // Should probably log this, but empty list is safe fallback
            };

            return respond(200, "Folder found", vec![], json!({
                "type": "folder",
                "id": r.id,
                "name": r.name,
                "files": files_data,
                "password_required": false,
                "expires_at": r.share_expires_at.map(|t| t.to_rfc3339())
            }));
        },
        Ok(None) => respond(404, "Invalid or expired link", vec!["Link not found".to_string()], json!({})),
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}

#[derive(Deserialize)]
pub struct PublicFetchChunksPayload {
    pub token: String, // Folder share token
    pub file_id: String, // File within that folder
    pub password: Option<String>,
}

pub async fn public_fetch_file_chunks(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<PublicFetchChunksPayload>,
) -> impl IntoResponse {
    let client_ip = crate::libs::geoip::client_ip(&headers, addr);
    let token = payload.token;
    let file_id = payload.file_id;
    let supplied_pw = payload.password.as_deref();

    // Verify folder token
    let folder = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(folder_rec)) = folder {
        if let Some(exp) = folder_rec.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        if let Some(ref required) = folder_rec.share_password_hash {
            // Meter before the KDF runs, not after.
            if !share_pw_allowed(&axum_state, &client_ip, &token) {
                return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
            }
            if !verify_share_pw(supplied_pw, required) {
                return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
            }
            if is_legacy_share_pw(required) {
                if let Some(pw) = supplied_pw {
                    upgrade_share_pw(&axum_state.pg_pool, "folders", &folder_rec.id, pw).await;
                }
            }
        }
        if folder_rec.share_type == Some("once".to_string()) {
            let downloads = folder_rec.link_downloads.unwrap_or(0);
            let max = folder_rec.link_max_downloads.unwrap_or(1);
            if downloads >= max {
                return respond(410, "This safe-link has expired.", vec![], json!({}));
            }
        }

        // Check if file is in this folder (direct child for now, recursive logic if needed later)
        // For simple structure (parent_id), we check if file.folder_id == folder.id
        // Doing a recursive check is expensive.
        // Simplification: We check if the file belongs to the same user as the folder.
        // AND ensuring the file is not deleted.
        // A slightly tighter check: Common Table Expression or just check simple parent for MVP.
        // Since `list_files` does not Recurse, `public_authorize_download` for folder will likely only list top-level files.
        
        let file = sqlx::query!(
            "SELECT id, user_id FROM files WHERE id = $1 AND folder_id = $2",
            file_id,
            folder_rec.id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;

        match file {
            Ok(Some(file_rec)) => {
                 // FETCH CHUNKS LOGIC (Duplicated from authorize file)
                let storage_type = if file_rec.user_id.is_some() { "sanctum" } else { "shadow" };

                let chunks_query = sqlx::query_as::<_, crate::models::Chunk>(
                    "SELECT * FROM chunks WHERE file_id = $1 AND uploaded = true ORDER BY chunk_index ASC",
                )
                .bind(file_rec.id.clone())
                .fetch_all(&axum_state.pg_pool)
                .await;

                 match chunks_query {
                    Ok(chunks) => {
                        let mut response_chunks = Vec::new();
                        for chunk in chunks {
                            match axum_state.r2.presigned_get_url(storage_type, &chunk.id).await {
                                Ok(url) => {
                                    response_chunks.push(PublicChunkResponse {
                                        id: chunk.id,
                                        index: chunk.chunk_index,
                                        size: chunk.size,
                                        presigned_url: url,
                                        file_offset: chunk.file_offset,
                                        checksum: chunk.checksum,
                                        nonce: chunk.nonce,
                                        salt: chunk.salt,
                                    });
                                },
                                Err(_e) => return respond(500, "Failed to generate download URL", vec![], json!({})),
                            }
                        }
                        
                        return respond(200, "Chunks retrieved", vec![], json!({
                            "type": "file_chunks",
                            "file_id": file_rec.id,
                            "chunks": response_chunks
                        }));
                    },
                    Err(_e) => return respond(500, "Failed to fetch chunks", vec![], json!({})),
                }

            },
            Ok(None) => return respond(404, "File not found in this folder", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

    } else {
        return respond(404, "Invalid folder token", vec![], json!({}));
    }
}


pub async fn public_authorize_download(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<PublicDownloadPayload>,
) -> impl IntoResponse {
    let client_ip = crate::libs::geoip::client_ip(&headers, addr);
    let token = payload.token;
    let supplied_pw = payload.password.as_deref();

    // Check file
    let file_query = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads, user_id, share_expires_at, share_password_hash FROM files WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = file_query {
        if let Some(exp) = r.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        if let Some(ref required) = r.share_password_hash {
            // Meter before the KDF runs, not after.
            if !share_pw_allowed(&axum_state, &client_ip, &token) {
                return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
            }
            if !verify_share_pw(supplied_pw, required) {
                return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
            }
            if is_legacy_share_pw(required) {
                if let Some(pw) = supplied_pw {
                    upgrade_share_pw(&axum_state.pg_pool, "files", &r.id, pw).await;
                }
            }
        }
        // Atomically enforce the once-limit and bump the counter in one
        // statement: a "public" link always passes; a "once" link passes only
        // while under its cap. Concurrent requests can't all slip through the
        // old check-then-increment race.
        let claim = sqlx::query!(
            "UPDATE files SET link_downloads = COALESCE(link_downloads, 0) + 1 \
             WHERE id = $1 AND (share_type <> 'once' OR COALESCE(link_downloads, 0) < COALESCE(link_max_downloads, 1)) \
             RETURNING id",
            r.id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;
        match claim {
            Ok(Some(_)) => {}
            Ok(None) => return respond(410, "This safe-link has expired.", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

        // FETCH CHUNKS LOGIC
        let storage_type = if r.user_id.is_some() { "sanctum" } else { "shadow" };

        let chunks_query = sqlx::query_as::<_, crate::models::Chunk>(
            "SELECT * FROM chunks WHERE file_id = $1 AND uploaded = true ORDER BY chunk_index ASC",
        )
        .bind(r.id.clone())
        .fetch_all(&axum_state.pg_pool)
        .await;

         match chunks_query {
            Ok(chunks) => {
                let mut response_chunks = Vec::new();
                for chunk in chunks {
                    match axum_state.r2.presigned_get_url(storage_type, &chunk.id).await {
                        Ok(url) => {
                            response_chunks.push(PublicChunkResponse {
                                id: chunk.id,
                                index: chunk.chunk_index,
                                size: chunk.size,
                                presigned_url: url,
                                file_offset: chunk.file_offset,
                                checksum: chunk.checksum,
                                nonce: chunk.nonce,
                                salt: chunk.salt,
                            });
                        },
                        Err(_e) => return respond(500, "Failed to generate download URL", vec![], json!({})),
                    }
                }
                
                return respond(200, "Download authorized", vec![], json!({
                    "type": "file",
                    "id": r.id,
                    "chunks": response_chunks
                }));
            },
            Err(_e) => return respond(500, "Failed to fetch chunks", vec![], json!({})),
        }
    }
    
    // Check folder
    let folder = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = folder {
        if let Some(exp) = r.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        if let Some(ref required) = r.share_password_hash {
            // Meter before the KDF runs, not after.
            if !share_pw_allowed(&axum_state, &client_ip, &token) {
                return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
            }
            if !verify_share_pw(supplied_pw, required) {
                return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
            }
            if is_legacy_share_pw(required) {
                if let Some(pw) = supplied_pw {
                    upgrade_share_pw(&axum_state.pg_pool, "folders", &r.id, pw).await;
                }
            }
        }
        // Atomically enforce the once-limit for the folder link (see the file
        // branch above).
        let claim = sqlx::query!(
            "UPDATE folders SET link_downloads = COALESCE(link_downloads, 0) + 1 \
             WHERE id = $1 AND (share_type <> 'once' OR COALESCE(link_downloads, 0) < COALESCE(link_max_downloads, 1)) \
             RETURNING id",
            r.id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;
        match claim {
            Ok(Some(_)) => {}
            Ok(None) => return respond(410, "This safe-link has expired.", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

        // Fetch files in folder (Non-recursive for now to match zip logic simply)
        let files = sqlx::query!(
            "SELECT id, name, size, mime, encrypted FROM files WHERE folder_id = $1 AND deleted = false",
            r.id
        )
        .fetch_all(&axum_state.pg_pool)
        .await;

        match files {
            Ok(file_list) => {
                 let files_data = file_list.into_iter().map(|f| json!({
                    "id": f.id,
                    "name": f.name,
                    "size": f.size,
                    "mime": f.mime,
                    "encrypted": f.encrypted
                })).collect::<Vec<_>>();

                return respond(200, "Download authorized", vec![], json!({ 
                    "type": "folder",
                    "folder_id": r.id, 
                    "files": files_data 
                }));
            },
            Err(_e) => return respond(500, "Failed to list folder files", vec![], json!({}))
        }
    }

    respond(404, "Invalid link", vec![], json!({}))
}

#[cfg(test)]
mod share_pw_tests {
    use super::*;

    #[test]
    fn argon2_roundtrip() {
        let stored = hash_share_pw("k7-Fern-Ridge-92").expect("hash");
        assert!(stored.starts_with("$argon2"));
        assert!(verify_share_pw(Some("k7-Fern-Ridge-92"), &stored));
        assert!(verify_share_pw(Some("  k7-Fern-Ridge-92  "), &stored), "trims");
        assert!(!verify_share_pw(Some("wrong"), &stored));
    }

    #[test]
    fn per_link_salt_means_distinct_hashes() {
        let a = hash_share_pw("same").unwrap();
        let b = hash_share_pw("same").unwrap();
        assert_ne!(a, b, "identical passwords must not share a digest");
    }

    #[test]
    fn legacy_sha256_still_verifies_and_is_flagged() {
        let legacy = hex::encode(Sha256::digest("old-password".as_bytes()));
        assert!(is_legacy_share_pw(&legacy));
        assert!(verify_share_pw(Some("old-password"), &legacy));
        assert!(!verify_share_pw(Some("nope"), &legacy));
        assert!(!is_legacy_share_pw(&hash_share_pw("x").unwrap()));
    }

    #[test]
    fn absent_or_blank_password_never_passes() {
        let stored = hash_share_pw("secret").unwrap();
        assert!(!verify_share_pw(None, &stored));
        assert!(!verify_share_pw(Some(""), &stored));
        assert!(!verify_share_pw(Some("   "), &stored));
    }
}
