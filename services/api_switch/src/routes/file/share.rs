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

/// The window both password meters count in.
const SHARE_PW_WINDOW: std::time::Duration = std::time::Duration::from_secs(300);

/// Throttle password attempts on a public share link.
///
/// Applied only once we know the link actually has a password, so ordinary
/// downloads are never throttled. Two buckets: per-IP stops one attacker, and
/// per-token stops a distributed attack converging on a single link. This is
/// also what keeps Argon2id from becoming a memory-exhaustion lever, so it must
/// run before any verification work.
///
/// The per-IP bucket counts every attempt, since every attempt costs a KDF run.
/// The per-token bucket counts only wrong passwords (see `check_share_pw`):
/// when it counted right ones too, a link passed round a team, or a recipient
/// retrying, hit the cap with nobody guessing at all, and a few guesses on top
/// of real use were enough to shut the recipient out. Twenty wrong passwords in
/// a window still close the link for the rest of that window.
fn share_pw_allowed(limiter: &crate::libs::ratelimit::RateLimiter, ip: &str, token: &str) -> bool {
    limiter.check(&format!("sharepw:{}", ip), 30, SHARE_PW_WINDOW)
        && limiter.allows(&format!("sharepwtok:{}", token), 20, SHARE_PW_WINDOW)
}

#[derive(Debug, PartialEq)]
enum SharePwRefusal {
    Throttled,
    Wrong,
}

/// Meter, then verify, a password supplied for a link. A wrong one is counted
/// against the link; a right one is not.
fn check_share_pw(
    limiter: &crate::libs::ratelimit::RateLimiter,
    ip: &str,
    token: &str,
    supplied: Option<&str>,
    stored: &str,
) -> Result<(), SharePwRefusal> {
    // Meter before the KDF runs, not after.
    if !share_pw_allowed(limiter, ip, token) {
        return Err(SharePwRefusal::Throttled);
    }
    if !verify_share_pw(supplied, stored) {
        limiter.hit(&format!("sharepwtok:{}", token), SHARE_PW_WINDOW);
        return Err(SharePwRefusal::Wrong);
    }
    Ok(())
}

/// Why fetch-chunks refuses a file from a shared folder.
#[derive(Debug, PartialEq)]
enum FolderFetchRefusal {
    Throttled,
    WrongPassword,
    Spent,
    AuthorizeRequired,
}

/// The folder a fetch-chunks call names, as far as deciding access goes.
struct FolderFetch<'a> {
    token: &'a str,
    one_time: bool,
    spent: bool,
    password_hash: Option<&'a str>,
}

/// Whether fetch-chunks may serve a file from `folder`. `granted` is whether
/// the caller presented a valid grant for it. Ok(true) means the link password
/// matched a legacy hash that should now be rewritten.
///
/// A valid grant stands in for the link password. Authorize issues one only
/// after checking the password, and the grant is bound to the stored hash, so
/// it proves no less than the password would. Checking the password on every
/// file as well put each one through the attempt meter, and a zip of more than
/// about twenty files came back short.
fn gate_folder_fetch(
    limiter: &crate::libs::ratelimit::RateLimiter,
    ip: &str,
    folder: &FolderFetch,
    supplied_pw: Option<&str>,
    granted: bool,
) -> Result<bool, FolderFetchRefusal> {
    if granted {
        return Ok(false);
    }
    let mut upgrade = false;
    if let Some(required) = folder.password_hash {
        match check_share_pw(limiter, ip, folder.token, supplied_pw, required) {
            Ok(()) => {}
            Err(SharePwRefusal::Throttled) => return Err(FolderFetchRefusal::Throttled),
            Err(SharePwRefusal::Wrong) => return Err(FolderFetchRefusal::WrongPassword),
        }
        upgrade = is_legacy_share_pw(required);
    }
    // A one-time folder is served only to whoever spent it. Checking the
    // counter here instead let anyone skip authorize and fetch every file
    // forever without spending the link, and refused the recipient who had
    // spent it, so the zip download always failed.
    if folder.one_time {
        return Err(if folder.spent { FolderFetchRefusal::Spent } else { FolderFetchRefusal::AuthorizeRequired });
    }
    Ok(upgrade)
}

/// Whether authorize hands back a grant for this folder: a one-time folder so
/// its files can be fetched after the spend, a password-protected one so they
/// can be fetched without a password check per file.
fn folder_grant_needed(one_time: bool, password_protected: bool) -> bool {
    one_time || password_protected
}

/// Rewrite a legacy SHA-256 link password as Argon2id after a successful check.
/// Best-effort: on failure the old hash stays and the next use retries.
/// Returns the hash now stored when the rewrite went through.
async fn upgrade_share_pw(pool: &sqlx::PgPool, table: &str, id: &str, pw: &str) -> Option<String> {
    let new_hash = hash_share_pw(pw.trim())?;
    let sql = if table == "files" {
        "UPDATE files SET share_password_hash = $1 WHERE id = $2"
    } else {
        "UPDATE folders SET share_password_hash = $1 WHERE id = $2"
    };
    match sqlx::query(sql).bind(&new_hash).bind(id).execute(pool).await {
        Ok(_) => Some(new_hash),
        Err(_) => None,
    }
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
    let (user_id, key_index) = match caller.as_ref().and_then(|c| Some((c.user_id.clone()?, c.api_key.clone()))) {
        Some(found) => found,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // `share_type` is the switch the owner actually sees, so it has to move the
    // column that grants access.
    //
    // `public_access` is what the non-token read paths (`fetch_chunks`,
    // `fetch_files`, `fetch_resource`) consult to serve a NON-owner who supplies
    // only the resource id. This used to be left untouched here, so a share the
    // owner had explicitly turned off stayed downloadable by id: the /s/<token>
    // page correctly refused (every token query filters `share_type != 'off'`)
    // while the id path kept serving. Turning sharing off has to revoke access,
    // not just retire the link, so the two move together now.
    //
    // Folders carry `public_access` too since 0045, so both branches move it.
    // While they did not, a folder was reachable only through the token path,
    // which is why an anonymous folder drop (no token, only an id) had no way to
    // be read by the person it was sent to.
    let publish = payload.share_type != "off";

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

        // Publishing hands out a link, so it must not land for a key the
        // account has dropped since this request was let in (libs::key_guard).
        match crate::libs::key_guard::still_current(&mut tx, &user_id, &key_index).await {
            Ok(true) => {}
            Ok(false) => return respond(401, "Unauthorized", vec!["Authentication required".to_string()], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

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
                // Turning sharing off drops the token as well. Keeping it would leave
                // a previously distributed URL able to come back to life the moment
                // sharing is re-enabled, which is not what "off" means to anyone.
                let token_to_set = if !publish {
                    None
                } else if record.share_token.is_none() {
                    Some(new_token)
                } else {
                    record.share_token // Keep existing
                };

                let result = sqlx::query!(
                    "UPDATE files SET share_type = $1, share_token = $2, public_access = $9, \
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
                    user_id,
                    publish
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

        // Publishing hands out a link, so it must not land for a key the
        // account has dropped since this request was let in (libs::key_guard).
        match crate::libs::key_guard::still_current(&mut tx, &user_id, &key_index).await {
            Ok(true) => {}
            Ok(false) => return respond(401, "Unauthorized", vec!["Authentication required".to_string()], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

        let current_folder = sqlx::query!(
            "SELECT share_token FROM folders WHERE id = $1 AND user_id = $2",
            folder_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await;
        
        match current_folder {
             Ok(Some(record)) => {
                let token_to_set = if !publish {
                    None
                } else if record.share_token.is_none() {
                    Some(generate_token())
                } else {
                    record.share_token
                };

                let result = sqlx::query!(
                    "UPDATE folders SET share_type = $1, share_token = $2, public_access = $3, \
                     share_expires_at = CASE WHEN $4 THEN $5::timestamptz ELSE share_expires_at END, \
                     share_password_hash = CASE WHEN $6 THEN $7::text ELSE share_password_hash END \
                     WHERE id = $8 AND user_id = $9 \
                     RETURNING share_token, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash",
                    payload.share_type,
                    token_to_set,
                    publish,
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
    let (user_id, key_index) = match caller.as_ref().and_then(|c| Some((c.user_id.clone()?, c.api_key.clone()))) {
        Some(found) => found,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // A new token is a link the caller can hand out, and a later toggle keeps
    // an existing token, so this is bound to the presenting key like a toggle.
    let mut tx = match axum_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };
    match crate::libs::key_guard::still_current(&mut tx, &user_id, &key_index).await {
        Ok(true) => {}
        Ok(false) => return respond(401, "Unauthorized", vec!["Authentication required".to_string()], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    }

    if let Some(file_id) = payload.file_id {
        let result = sqlx::query!(
            "UPDATE files SET share_token = $1, link_downloads = 0 WHERE id = $2 AND user_id = $3 RETURNING share_token",
            new_token,
            file_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await;

        match result {
            Ok(Some(r)) => match tx.commit().await {
                Ok(()) => respond(200, "Token regenerated", vec![], json!({ "share_token": r.share_token })),
                Err(_e) => respond(500, "Database error", vec![], json!({})),
            },
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
        .fetch_optional(&mut *tx)
        .await;

        match result {
            Ok(Some(r)) => match tx.commit().await {
                Ok(()) => respond(200, "Token regenerated", vec![], json!({ "share_token": r.share_token })),
                Err(_e) => respond(500, "Database error", vec![], json!({})),
            },
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

/// Record a delivery receipt: an authorized access to a shared link. Best effort,
/// so a logging failure never blocks the download.
async fn record_share_access(
    pool: &sqlx::PgPool,
    token: &str,
    file_id: &str,
    owner_user_id: &Option<String>,
    ip: &str,
    user_agent: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO share_access_events (share_token, file_id, owner_user_id, ip, user_agent) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(token)
    .bind(file_id)
    .bind(owner_user_id)
    .bind(ip)
    .bind(user_agent)
    .execute(pool)
    .await;
}

#[derive(Deserialize)]
pub struct AccessLogPayload {
    pub id: String, // a file or folder id the caller owns
}

/// Owner-only: the delivery receipts (who accessed the link, and when) for one of
/// the caller's shared files or folders. The "proof of delivery" surface that
/// makes secure delivery a paid feature.
pub async fn access_log(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<AccessLogPayload>,
) -> impl IntoResponse {
    let caller = match caller.as_ref() {
        Some(c) => c,
        None => return respond(401, "Unauthorized", vec!["Authentication required".to_string()], json!({})),
    };

    // Resolve the shared resource's token, checking ownership (file, then folder).
    let file = sqlx::query!(
        "SELECT share_token, user_id, owner_api_key FROM files WHERE id = $1",
        payload.id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;
    let token: Option<String> = match file {
        Ok(Some(f)) if caller.owns(&f.user_id, &f.owner_api_key) => f.share_token,
        Ok(Some(_)) => return respond(404, "Not found", vec![], json!({})),
        _ => {
            let folder = sqlx::query!(
                "SELECT share_token, user_id, owner_api_key FROM folders WHERE id = $1",
                payload.id
            )
            .fetch_optional(&axum_state.pg_pool)
            .await;
            match folder {
                Ok(Some(fo)) if caller.owns(&fo.user_id, &fo.owner_api_key) => fo.share_token,
                _ => return respond(404, "Not found", vec![], json!({})),
            }
        }
    };

    // Ownership is confirmed. Delivery receipts are a paid feature (the access
    // history is recorded for everyone, so it is all there the moment a free user
    // upgrades). A non-owner already got a 404 above, so this cannot leak.
    if !crate::libs::plans::is_paid(&axum_state.pg_pool, &caller.user_id.clone().unwrap_or_default()).await {
        return respond(
            403,
            "Upgrade required",
            vec!["Delivery receipts are a paid feature. Upgrade to see who opened your links, and when.".to_string()],
            json!({ "upgrade_required": true, "feature": "delivery_receipts" }),
        );
    }

    let token = match token {
        Some(t) => t,
        None => return respond(200, "No share", vec![], json!({ "events": [], "count": 0 })),
    };

    let events = sqlx::query!(
        "SELECT ip, user_agent, created_at FROM share_access_events \
         WHERE share_token = $1 ORDER BY created_at DESC LIMIT 100",
        token
    )
    .fetch_all(&axum_state.pg_pool)
    .await;

    match events {
        Ok(rows) => {
            let list: Vec<_> = rows
                .into_iter()
                .map(|e| json!({ "ip": e.ip, "user_agent": e.user_agent, "at": e.created_at }))
                .collect();
            let count = list.len();
            respond(200, "Access log", vec![], json!({ "events": list, "count": count }))
        }
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}

pub async fn public_get_info(
    State(axum_state): State<crate::AppState>,
    Path(token): Path<String>,
) -> impl IntoResponse {
    // Check files
    let file = sqlx::query!(
        "SELECT id, name, size, mime, share_type, link_downloads, link_max_downloads, encrypted, share_expires_at, share_password_hash FROM files WHERE share_token = $1 AND share_type != 'off' AND deleted = false",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file {
        Ok(Some(r)) => {
            // A link to something inside a trashed folder is gone with it.
            if crate::libs::sharing::in_trashed_folder(&axum_state.pg_pool, crate::libs::sharing::Shared::File(&r.id)).await {
                return respond(404, "Invalid or expired link", vec!["Link not found".to_string()], json!({}));
            }
            if let Some(exp) = r.share_expires_at {
                if exp < Utc::now() {
                    return respond(410, "This link has expired.", vec!["Link expired".to_string()], json!({}));
                }
            }
            if r.share_type == Some("once".to_string()) {
                let downloads = r.link_downloads.unwrap_or(0);
                let max = r.link_max_downloads.unwrap_or(1);
                if downloads >= max {
                    return respond(410, "This one-time link has already been used.", vec!["Link limit reached".to_string()], json!({}));
                }
            }

            // A password-protected link discloses nothing before the password is
            // supplied. Filenames and sizes are frequently the sensitive part of
            // a share, so reporting `password_required` alongside the metadata
            // (as this used to) defeated the point of setting a password.
            // Authorization is what spends a one-time link, and it happens before
            // a single byte is transferred, so a dropped connection or a closed
            // tab consumes the recipient's only attempt. Moving the spend to a
            // post-download confirmation would let a client simply never confirm
            // and download forever, so the counter stays where it is and the page
            // warns instead: the recipient can at least make sure they are ready.
            let one_shot = r.share_type.as_deref() == Some("once");
            let remaining = if one_shot {
                Some((r.link_max_downloads.unwrap_or(1) - r.link_downloads.unwrap_or(0)).max(0))
            } else {
                None
            };

            if r.share_password_hash.is_some() {
                return respond(200, "Password required", vec![], json!({
                    "type": "file",
                    "password_required": true,
                    "sender_paid": crate::libs::plans::sender_paid(&axum_state.pg_pool, crate::libs::plans::SharedBy::Token(&token)).await,
                    "one_time": one_shot,
                    "remaining": remaining
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
                "sender_paid": crate::libs::plans::sender_paid(&axum_state.pg_pool, crate::libs::plans::SharedBy::Token(&token)).await,
                "one_time": one_shot,
                "remaining": remaining,
                "expires_at": r.share_expires_at.map(|t| t.to_rfc3339())
            }));
        },
        Ok(None) => {},
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    }

    // Check folders
    let folder = sqlx::query!(
        "SELECT id, name, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off' AND deleted = false",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder {
        Ok(Some(r)) => {
            if crate::libs::sharing::in_trashed_folder(&axum_state.pg_pool, crate::libs::sharing::Shared::Folder(&r.id)).await {
                return respond(404, "Invalid or expired link", vec!["Link not found".to_string()], json!({}));
            }
            if let Some(exp) = r.share_expires_at {
                if exp < Utc::now() {
                    return respond(410, "This link has expired.", vec!["Link expired".to_string()], json!({}));
                }
            }
            let one_shot = r.share_type.as_deref() == Some("once");
            let remaining = if one_shot {
                Some((r.link_max_downloads.unwrap_or(1) - r.link_downloads.unwrap_or(0)).max(0))
            } else {
                None
            };
            if remaining == Some(0) {
                return respond(410, "This safe-link has expired.", vec!["Link limit reached".to_string()], json!({}));
            }

            // As above: a protected folder listing is withheld entirely until the
            // password is verified. This was the more serious of the two leaks,
            // since it exposed every filename in the folder.
            if r.share_password_hash.is_some() {
                return respond(200, "Password required", vec![], json!({
                    "type": "folder",
                    "password_required": true,
                    "one_time": one_shot,
                    "remaining": remaining
                }));
            }

            // The whole subtree. This listed direct children only, so sharing a
            // folder with any structure to it showed the recipient the top level
            // and silently dropped everything in a subdirectory.
            let subtree = crate::libs::sharing::folder_subtree(&axum_state.pg_pool, &r.id).await;
            let files_data = match subtree {
                Ok(tree) => match crate::libs::sharing::subtree_files(&axum_state.pg_pool, &tree).await {
                    Ok(list) => list.into_iter().map(|(f, path)| json!({
                        "id": f.id,
                        "name": f.name,
                        "size": f.size,
                        "mime": f.mime,
                        "encrypted": f.encrypted,
                        "path": path
                    })).collect::<Vec<_>>(),
                    Err(_) => vec![] // Should probably log this, but empty list is safe fallback
                },
                Err(_) => vec![]
            };

            return respond(200, "Folder found", vec![], json!({
                "type": "folder",
                "id": r.id,
                "name": r.name,
                "files": files_data,
                "password_required": false,
                "sender_paid": crate::libs::plans::sender_paid(&axum_state.pg_pool, crate::libs::plans::SharedBy::Token(&token)).await,
                "one_time": one_shot,
                "remaining": remaining,
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
    // Issued by authorize when it spends a one-time folder link. Ignored for
    // other share types.
    pub grant: Option<String>,
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
        "SELECT id, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off' AND deleted = false",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(folder_rec)) = folder {
        if crate::libs::sharing::in_trashed_folder(&axum_state.pg_pool, crate::libs::sharing::Shared::Folder(&folder_rec.id)).await {
            return respond(404, "Invalid folder token", vec![], json!({}));
        }
        if let Some(exp) = folder_rec.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        let one_time = folder_rec.share_type.as_deref() == Some("once");
        let granted = payload.grant.as_deref().map_or(false, |g| {
            crate::libs::share_grant::verify(
                g,
                crate::libs::share_grant::GrantScope {
                    folder_id: &folder_rec.id,
                    token: &token,
                    password_hash: folder_rec.share_password_hash.as_deref(),
                },
            )
        });
        let gate = FolderFetch {
            token: &token,
            one_time,
            spent: folder_rec.link_downloads.unwrap_or(0) >= folder_rec.link_max_downloads.unwrap_or(1),
            password_hash: folder_rec.share_password_hash.as_deref(),
        };
        match gate_folder_fetch(&axum_state.rate_limiter, &client_ip, &gate, supplied_pw, granted) {
            Ok(upgrade) => {
                if let (true, Some(pw)) = (upgrade, supplied_pw) {
                    let _ = upgrade_share_pw(&axum_state.pg_pool, "folders", &folder_rec.id, pw).await;
                }
            }
            Err(FolderFetchRefusal::Throttled) => {
                return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
            }
            Err(FolderFetchRefusal::WrongPassword) => {
                return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
            }
            Err(FolderFetchRefusal::Spent) => {
                return respond(410, "This safe-link has expired.", vec![], json!({}));
            }
            Err(FolderFetchRefusal::AuthorizeRequired) => {
                return respond(
                    403,
                    "Download not authorized",
                    vec!["authorize_required".to_string()],
                    json!({ "authorize_required": true }),
                );
            }
        }

        // The file has to be inside the shared folder, at any depth. Matching
        // only `folder_id = folder.id` meant the listing and the download
        // disagreed about what the share contained.
        let file =
            crate::libs::sharing::file_in_subtree(&axum_state.pg_pool, &folder_rec.id, &file_id)
                .await;

        match file {
            Ok(Some((file_id, file_user_id))) => {
                 // FETCH CHUNKS LOGIC (Duplicated from authorize file)
                let storage_type = if file_user_id.is_some() { "sanctum" } else { "shadow" };

                let chunks_query = sqlx::query_as::<_, crate::models::Chunk>(
                    "SELECT * FROM chunks WHERE file_id = $1 AND uploaded = true ORDER BY chunk_index ASC",
                )
                .bind(file_id.clone())
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

                        // Delivery receipt, one per file handed out, as the file
                        // branch of authorize records for a file link (best
                        // effort). Written once the URLs exist, so a fetch that
                        // fails above leaves no receipt for a file never served.
                        record_share_access(
                            &axum_state.pg_pool,
                            &token,
                            &file_id,
                            &file_user_id,
                            &client_ip,
                            headers.get("user-agent").and_then(|v| v.to_str().ok()),
                        )
                        .await;

                        return respond(200, "Chunks retrieved", vec![], json!({
                            "type": "file_chunks",
                            "file_id": file_id,
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


/// What authorize answers for a file link.
///
/// info says nothing about a password-protected file, so this is where the
/// share page first learns what it is downloading, and above all whether to
/// decrypt it. Without `encrypted` here it saved the ciphertext as the file.
fn authorized_file_data(
    id: &str,
    name: &str,
    size: i64,
    mime: &str,
    encrypted: bool,
    chunks: &[PublicChunkResponse],
) -> serde_json::Value {
    json!({
        "type": "file",
        "id": id,
        "name": name,
        "size": size,
        "mime": mime,
        "encrypted": encrypted,
        "chunks": chunks
    })
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
        "SELECT id, name, size, mime, encrypted, share_type, link_downloads, link_max_downloads, user_id, share_expires_at, share_password_hash FROM files WHERE share_token = $1 AND share_type != 'off' AND deleted = false",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = file_query {
        // Before the password meter and the one-time spend: a link inside a
        // trashed folder is gone, not merely refused.
        if crate::libs::sharing::in_trashed_folder(&axum_state.pg_pool, crate::libs::sharing::Shared::File(&r.id)).await {
            return respond(404, "Invalid link", vec![], json!({}));
        }
        if let Some(exp) = r.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        if let Some(ref required) = r.share_password_hash {
            match check_share_pw(&axum_state.rate_limiter, &client_ip, &token, supplied_pw, required) {
                Ok(()) => {}
                Err(SharePwRefusal::Throttled) => {
                    return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
                }
                Err(SharePwRefusal::Wrong) => {
                    return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
                }
            }
            if is_legacy_share_pw(required) {
                if let Some(pw) = supplied_pw {
                    let _ = upgrade_share_pw(&axum_state.pg_pool, "files", &r.id, pw).await;
                }
            }
        }
        // Atomically enforce the once-limit and bump the counter in one
        // statement: a "public" link always passes; a "once" link passes only
        // while under its cap. Concurrent requests can't all slip through the
        // old check-then-increment race.
        //
        // Note that this spends the link at AUTHORIZATION, before any bytes have
        // moved, so an interrupted transfer costs the recipient their only
        // attempt. That is a real problem (silo-suggests.md S2-17) but not one to
        // fix by loosening this: chunks are served from R2 by presigned URL, so
        // the server never observes the transfer and "exactly once" can only mean
        // "exactly one authorization". Anything that lets a second authorize
        // through weakens a guarantee the product sells, and is the owner's call,
        // not a refactor. The recipient's way out is a fresh link from the owner.
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
            Ok(None) => return respond(410, "This one-time link has already been used.", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

        // Delivery receipt: log this authorized access for the owner (best effort).
        record_share_access(
            &axum_state.pg_pool,
            &token,
            &r.id,
            &r.user_id,
            &client_ip,
            headers.get("user-agent").and_then(|v| v.to_str().ok()),
        )
        .await;

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
                
                return respond(
                    200,
                    "Download authorized",
                    vec![],
                    authorized_file_data(&r.id, &r.name, r.size, &r.mime, r.encrypted, &response_chunks),
                );
            },
            Err(_e) => return respond(500, "Failed to fetch chunks", vec![], json!({})),
        }
    }
    
    // Check folder
    let folder = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads, share_expires_at, share_password_hash FROM folders WHERE share_token = $1 AND share_type != 'off' AND deleted = false",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = folder {
        if crate::libs::sharing::in_trashed_folder(&axum_state.pg_pool, crate::libs::sharing::Shared::Folder(&r.id)).await {
            return respond(404, "Invalid link", vec![], json!({}));
        }
        if let Some(exp) = r.share_expires_at {
            if exp < Utc::now() {
                return respond(410, "This link has expired.", vec![], json!({}));
            }
        }
        // The link password hash as stored once this call is done, which is what
        // fetch-chunks will check a grant against.
        let mut stored_pw_hash = r.share_password_hash.clone();
        if let Some(ref required) = r.share_password_hash {
            match check_share_pw(&axum_state.rate_limiter, &client_ip, &token, supplied_pw, required) {
                Ok(()) => {}
                Err(SharePwRefusal::Throttled) => {
                    return respond(429, "Too many attempts", vec!["Too many password attempts for this link. Try again in a few minutes.".to_string()], json!({ "password_required": true }));
                }
                Err(SharePwRefusal::Wrong) => {
                    return respond(401, "Incorrect password", vec!["password_required".to_string()], json!({ "password_required": true }));
                }
            }
            if is_legacy_share_pw(required) {
                if let Some(pw) = supplied_pw {
                    if let Some(new_hash) = upgrade_share_pw(&axum_state.pg_pool, "folders", &r.id, pw).await {
                        stored_pw_hash = Some(new_hash);
                    }
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
            Ok(None) => return respond(410, "This one-time link has already been used.", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }

        // The whole subtree, matching what the listing shows and what the zip on
        // the other end is about to rebuild.
        let files = match crate::libs::sharing::folder_subtree(&axum_state.pg_pool, &r.id).await {
            Ok(tree) => crate::libs::sharing::subtree_files(&axum_state.pg_pool, &tree).await,
            Err(e) => Err(e),
        };

        match files {
            Ok(file_list) => {
                let total_bytes: i64 = file_list.iter().map(|(f, _)| f.size.max(0)).fold(0, i64::saturating_add);
                 let files_data = file_list.into_iter().map(|(f, path)| json!({
                    "id": f.id,
                    "name": f.name,
                    "size": f.size,
                    "mime": f.mime,
                    "encrypted": f.encrypted,
                    "path": path
                })).collect::<Vec<_>>();

                // A one-time folder's files are fetched one by one after this, by
                // which point the counter reads spent. The grant is what lets this
                // caller, and only this caller, finish. On a password-protected
                // folder it also saves a password check, and a slot of the attempt
                // meter, per file. It lasts as long as a folder of this size needs
                // (see share_grant::ttl_for).
                let grant = if folder_grant_needed(r.share_type.as_deref() == Some("once"), stored_pw_hash.is_some()) {
                    let scope = crate::libs::share_grant::GrantScope {
                        folder_id: &r.id,
                        token: &token,
                        password_hash: stored_pw_hash.as_deref(),
                    };
                    match crate::libs::share_grant::issue(scope, total_bytes) {
                        Some(g) => Some(g),
                        None => return respond(500, "Download grant unavailable", vec![], json!({})),
                    }
                } else {
                    None
                };

                return respond(200, "Download authorized", vec![], json!({
                    "type": "folder",
                    "folder_id": r.id,
                    "files": files_data,
                    "grant": grant
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

    /// info withholds everything about a password-protected file, so the
    /// authorize answer is the only place the share page can learn whether to
    /// decrypt. Without `encrypted` it saved raw ciphertext.
    #[test]
    fn authorize_tells_the_page_what_it_is_downloading() {
        let chunks = vec![PublicChunkResponse {
            id: "c1".into(),
            index: 0,
            size: 1040,
            presigned_url: "https://r2.example/c1".into(),
            file_offset: 0,
            checksum: "ab".into(),
            nonce: Some("bm9uY2U=".into()),
            salt: Some("c2FsdA==".into()),
        }];
        let data = authorized_file_data("f1", "tax-2026.pdf", 1024, "application/pdf", true, &chunks);
        assert_eq!(data["type"], "file");
        assert_eq!(data["id"], "f1");
        assert_eq!(data["encrypted"], true);
        assert_eq!(data["name"], "tax-2026.pdf");
        assert_eq!(data["size"], 1024);
        assert_eq!(data["mime"], "application/pdf");
        assert_eq!(data["chunks"][0]["salt"], "c2FsdA==");

        let plain = authorized_file_data("f2", "a.txt", 3, "text/plain", false, &[]);
        assert_eq!(plain["encrypted"], false);
    }

    #[test]
    fn absent_or_blank_password_never_passes() {
        let stored = hash_share_pw("secret").unwrap();
        assert!(!verify_share_pw(None, &stored));
        assert!(!verify_share_pw(Some(""), &stored));
        assert!(!verify_share_pw(Some("   "), &stored));
    }
}

#[cfg(test)]
mod folder_fetch_tests {
    use super::*;
    use crate::libs::ratelimit::RateLimiter;

    const IP: &str = "203.0.113.9";
    const TOKEN: &str = "tok-folder";

    fn folder<'a>(one_time: bool, spent: bool, hash: Option<&'a str>) -> FolderFetch<'a> {
        FolderFetch { token: TOKEN, one_time, spent, password_hash: hash }
    }

    /// A zip is one authorize and then one fetch-chunks per file. Each of those
    /// used to take a slot of the password meter, so a 24 file one-time folder
    /// got 19 files and 429 for the rest, from a link that was now spent.
    #[test]
    fn a_granted_zip_of_a_one_time_password_folder_gets_every_file() {
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        assert!(share_pw_allowed(&limiter, IP, TOKEN), "authorize");
        for i in 0..24 {
            let got = gate_folder_fetch(&limiter, IP, &folder(true, true, Some(&hash)), Some("k7-Fern-Ridge-92"), true);
            assert_eq!(got, Ok(false), "file {i} of 24");
        }
    }

    #[test]
    fn a_granted_zip_of_a_public_password_folder_gets_every_file() {
        assert!(folder_grant_needed(false, true), "authorize must hand a password folder a grant");
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        assert!(share_pw_allowed(&limiter, IP, TOKEN), "authorize");
        for i in 0..24 {
            let got = gate_folder_fetch(&limiter, IP, &folder(false, false, Some(&hash)), Some("k7-Fern-Ridge-92"), true);
            assert_eq!(got, Ok(false), "file {i} of 24");
        }
    }

    #[test]
    fn folders_without_a_password_or_a_spend_get_no_grant() {
        assert!(folder_grant_needed(true, false));
        assert!(folder_grant_needed(true, true));
        assert!(!folder_grant_needed(false, false));
    }

    #[test]
    fn guesses_without_a_grant_are_still_metered() {
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        for _ in 0..20 {
            let got = gate_folder_fetch(&limiter, IP, &folder(false, false, Some(&hash)), Some("guess"), false);
            assert_eq!(got, Err(FolderFetchRefusal::WrongPassword));
        }
        let got = gate_folder_fetch(&limiter, IP, &folder(false, false, Some(&hash)), Some("k7-Fern-Ridge-92"), false);
        assert_eq!(got, Err(FolderFetchRefusal::Throttled));
    }

    /// Only wrong guesses count against the link. Right answers used to take a
    /// slot too, so a recipient on a busy link was refused like a guesser.
    #[test]
    fn right_passwords_without_a_grant_are_not_metered_against_the_link() {
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        let f = folder(false, false, Some(&hash));
        for i in 0..25 {
            let got = gate_folder_fetch(&limiter, IP, &f, Some("k7-Fern-Ridge-92"), false);
            assert_eq!(got, Ok(false), "right answer {i} of 25");
        }
        // And a few wrong ones in between still leave room for the right one.
        for _ in 0..5 {
            assert_eq!(gate_folder_fetch(&limiter, "198.51.100.7", &f, Some("guess"), false), Err(FolderFetchRefusal::WrongPassword));
        }
        assert_eq!(gate_folder_fetch(&limiter, "198.51.100.8", &f, Some("k7-Fern-Ridge-92"), false), Ok(false));
    }

    #[test]
    fn a_password_folder_without_a_grant_still_needs_the_password() {
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        let f = folder(false, false, Some(&hash));
        assert_eq!(gate_folder_fetch(&limiter, IP, &f, None, false), Err(FolderFetchRefusal::WrongPassword));
        assert_eq!(gate_folder_fetch(&limiter, IP, &f, Some("k7-Fern-Ridge-92"), false), Ok(false));
    }

    #[test]
    fn a_one_time_folder_without_a_grant_is_refused_even_with_the_password() {
        let limiter = RateLimiter::new();
        let hash = hash_share_pw("k7-Fern-Ridge-92").unwrap();
        let pw = Some("k7-Fern-Ridge-92");
        assert_eq!(gate_folder_fetch(&limiter, IP, &folder(true, false, Some(&hash)), pw, false), Err(FolderFetchRefusal::AuthorizeRequired));
        assert_eq!(gate_folder_fetch(&limiter, IP, &folder(true, true, Some(&hash)), pw, false), Err(FolderFetchRefusal::Spent));
        assert_eq!(gate_folder_fetch(&limiter, IP, &folder(true, true, None), None, false), Err(FolderFetchRefusal::Spent));
        assert_eq!(gate_folder_fetch(&limiter, IP, &folder(true, true, None), None, true), Ok(false));
    }

    #[test]
    fn a_legacy_hash_is_flagged_for_upgrade() {
        let limiter = RateLimiter::new();
        let legacy = hex::encode(Sha256::digest("old-password".as_bytes()));
        let got = gate_folder_fetch(&limiter, IP, &folder(false, false, Some(&legacy)), Some("old-password"), false);
        assert_eq!(got, Ok(true));
    }
}

#[cfg(test)]
mod trashed_parent_tests {
    //! These call the handlers against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::extract::ConnectInfo;
    use axum::http::{HeaderMap, StatusCode};

    fn peer() -> ConnectInfo<std::net::SocketAddr> {
        ConnectInfo(std::net::SocketAddr::from(([127, 0, 0, 1], 40000)))
    }

    async fn folder(pool: &sqlx::PgPool, user: &str, parent: Option<&str>, token: Option<&str>) -> String {
        sqlx::query_scalar(
            "INSERT INTO folders (name, user_id, parent_id, share_token, share_type) \
             VALUES ('dir', $1, $2, $3, CASE WHEN $3::text IS NULL THEN NULL ELSE 'public' END) RETURNING id",
        )
        .bind(user)
        .bind(parent)
        .bind(token)
        .fetch_one(pool)
        .await
        .expect("seed folder")
    }

    async fn file(pool: &sqlx::PgPool, user: &str, folder_id: &str, token: Option<&str>) -> String {
        sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, \
             sha256_checksum, blake3_checksum, public_access, folder_id, share_token, share_type) \
             VALUES ($1, 'f.bin', 'application/octet-stream', 1, false, 1, 1, 's', '', false, $2, $3, \
             CASE WHEN $3::text IS NULL THEN NULL ELSE 'public' END) RETURNING id",
        )
        .bind(user)
        .bind(folder_id)
        .bind(token)
        .fetch_one(pool)
        .await
        .expect("seed file")
    }

    async fn info(st: &crate::AppState, token: &str) -> StatusCode {
        public_get_info(State(st.clone()), Path(token.to_string())).await.into_response().status()
    }

    async fn authorize(st: &crate::AppState, token: &str) -> StatusCode {
        let payload = PublicDownloadPayload { token: token.to_string(), password: None };
        public_authorize_download(peer(), State(st.clone()), HeaderMap::new(), Json(payload))
            .await
            .into_response()
            .status()
    }

    async fn fetch(st: &crate::AppState, token: &str, file_id: &str) -> StatusCode {
        let payload = PublicFetchChunksPayload {
            token: token.to_string(),
            file_id: file_id.to_string(),
            password: None,
            grant: None,
        };
        public_fetch_file_chunks(peer(), State(st.clone()), HeaderMap::new(), Json(payload))
            .await
            .into_response()
            .status()
    }

    async fn cleanup(pool: &sqlx::PgPool, user: &str) {
        let _ = sqlx::query("DELETE FROM share_access_events WHERE owner_user_id = $1").bind(user).execute(pool).await;
        let _ = sqlx::query("DELETE FROM files WHERE user_id = $1").bind(user).execute(pool).await;
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(user).execute(pool).await;
        forget(pool, user).await;
    }

    /// The owner shares a file and a subfolder that sit two levels down in a
    /// folder, then puts the top folder in the trash. Trashing marks only that
    /// one row, so the token paths, which looked at the shared row alone, kept
    /// serving both links.
    #[tokio::test]
    async fn links_inside_a_trashed_folder_stop_working() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let pool = &st.pg_pool;
        let (user, _key) = account(pool).await;
        let file_token = generate_token();
        let dir_token = generate_token();

        let top = folder(pool, &user, None, None).await;
        let mid = folder(pool, &user, Some(&top), None).await;
        let shared_file = file(pool, &user, &mid, Some(&file_token)).await;
        let shared_dir = folder(pool, &user, Some(&mid), Some(&dir_token)).await;
        let inner = file(pool, &user, &shared_dir, None).await;

        let before = (
            info(&st, &file_token).await,
            authorize(&st, &file_token).await,
            info(&st, &dir_token).await,
            authorize(&st, &dir_token).await,
            fetch(&st, &dir_token, &inner).await,
        );

        sqlx::query("UPDATE folders SET deleted = true, deleted_on = NOW() WHERE id = $1")
            .bind(&top)
            .execute(pool)
            .await
            .expect("trash top folder");

        let after = (
            info(&st, &file_token).await,
            authorize(&st, &file_token).await,
            info(&st, &dir_token).await,
            authorize(&st, &dir_token).await,
            fetch(&st, &dir_token, &inner).await,
        );
        let _ = shared_file;
        cleanup(pool, &user).await;

        let ok = StatusCode::OK;
        assert_eq!(before, (ok, ok, ok, ok, ok), "live links work before the trash");
        let gone = StatusCode::NOT_FOUND;
        assert_eq!(after, (gone, gone, gone, gone, gone), "links under a trashed folder");
    }

    /// A recipient who knows the password downloads a protected file more than
    /// twenty times in five minutes (a team passing the link round, or reloads).
    /// Right answers used to take a slot of the per-link meter, so the 21st got
    /// 429 just as if someone were guessing.
    #[tokio::test]
    async fn right_passwords_do_not_use_up_the_link_meter() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let pool = &st.pg_pool;
        let (user, _key) = account(pool).await;
        let token = generate_token();
        let dir = folder(pool, &user, None, None).await;
        let id = file(pool, &user, &dir, Some(&token)).await;
        sqlx::query("UPDATE files SET share_password_hash = $1 WHERE id = $2")
            .bind(hash_share_pw("k7-Fern-Ridge-92").unwrap())
            .bind(&id)
            .execute(pool)
            .await
            .expect("set link password");

        let mut statuses = vec![];
        for i in 0..25u8 {
            let mut headers = HeaderMap::new();
            // A different address each time, so only the per-link meter is in play.
            headers.insert("x-client-ip", format!("198.51.100.{}", i + 1).parse().unwrap());
            let payload = PublicDownloadPayload { token: token.clone(), password: Some("k7-Fern-Ridge-92".into()) };
            let status = public_authorize_download(peer(), State(st.clone()), headers, Json(payload))
                .await
                .into_response()
                .status();
            statuses.push(status);
        }
        cleanup(pool, &user).await;

        assert!(statuses.iter().all(|s| *s == StatusCode::OK), "{statuses:?}");
    }
}
