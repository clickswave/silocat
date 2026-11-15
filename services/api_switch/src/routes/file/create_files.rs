use crate::libs;
use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;
use std::net::SocketAddr;

#[derive(serde::Deserialize, Debug)]
pub struct ChunkData {
    pub start: i64,
    pub end: i64,
    pub size: i64,
    pub checksum: String,
    pub salt: Option<String>,
    pub nonce: Option<String>,
}

#[derive(serde::Deserialize, Debug)]
pub struct PayloadBody {
    pub file_encrypted: bool,
    pub file_name: String,
    pub file_mime: String,
    pub file_size: i64,
    pub chunks: Vec<ChunkData>,
    pub sha256_checksum: String,
    pub blake3_checksum: String,
    pub folder_id: Option<String>,
}

pub async fn handle(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    headers: HeaderMap,
    Json(payload): Json<PayloadBody>,
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

    // Ban / restrict gate before any upload metadata is created.
    match &caller.user_id {
        Some(uid) => {
            if let Some(ban) = libs::bans::user_ban(&axum_state.pg_pool, uid).await {
                return respond(
                    403,
                    "Account banned",
                    vec![ban.reason.clone().unwrap_or_else(|| "Your account has been banned.".to_string())],
                    json!({ "banned": true, "reason": ban.reason, "until": ban.until }),
                );
            }
            if libs::bans::user_restricted(&axum_state.pg_pool, uid).await {
                return respond(
                    403,
                    "Account restricted",
                    vec!["Your account is restricted; uploads are disabled.".to_string()],
                    json!({ "restricted": true }),
                );
            }
        }
        None => {
            let ip = libs::geoip::client_ip(&headers, addr);
            if let Some(ban) = libs::bans::ip_ban(&axum_state.pg_pool, &ip).await {
                return respond(
                    403,
                    "You are banned",
                    vec![ban.reason.clone().unwrap_or_else(|| "Your access has been banned.".to_string())],
                    json!({ "banned": true, "reason": ban.reason, "until": ban.until }),
                );
            }
        }
    }

    // The declared sizes decide quota admission below and each chunk's signed
    // PUT length, so they have to describe a real file: no negative sizes, no
    // chunk over the largest any client sends, and chunks that add up.
    let chunk_sizes: Vec<i64> = payload.chunks.iter().map(|c| c.size).collect();
    if let Err(why) = libs::uploads::check_declared(payload.file_size, &chunk_sizes) {
        return respond(400, "Invalid upload", vec![why.to_string()], json!({}));
    }
    // The name ends up as a zip entry when a folder is downloaded, so it has to
    // be a single path segment.
    if let Err(why) = libs::input_validators::entry_name(&payload.file_name) {
        return respond(400, "Invalid file name", vec![why], json!({}));
    }

    // A destination folder, if given, must belong to the caller: no planting
    // files into another user's folder.
    if let Some(folder_id) = &payload.folder_id {
        let parent = sqlx::query!(
            "SELECT user_id, owner_api_key FROM folders WHERE id = $1",
            folder_id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;
        match parent {
            Ok(Some(rec)) if caller.owns(&rec.user_id, &rec.owner_api_key) => {}
            Ok(_) => return respond(404, "Destination folder not found", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }
    }

    let owner_api_key = caller.api_key.clone();

    // A file in a registered user's drive is private on creation, full stop.
    //
    // `public_access` is what `fetch_chunks` / `fetch_files` / `fetch_resource`
    // consult to hand presigned download URLs to a NON-owner who supplies only
    // the file id. The web client used to send `public_access: !encrypted`, so
    // every unencrypted upload into someone's private drive was readable by id
    // by anyone, with no share link ever created. Ids are UUIDv4 so they are not
    // guessable, but that is a capability that survives referrer leaks, logs and
    // support tickets, and it is not what the drive UI promises.
    //
    // Publishing is now exclusively the job of `share::toggle_share`, which the
    // owner drives explicitly and which can be turned back off.
    //
    // An anonymous drop is the mirror image: it has no drive to sit in and no
    // owner who can log in to publish it later, so the id IS the capability and
    // the file has to be readable by it. That half used to be trusted from the
    // body, and the drop page sent `public_access: !encrypted`. So turning on a
    // password, the one thing that makes a drop safe to hand out, was also what
    // made its link 404 for everyone including the uploader. Both halves are now
    // decided here, from the caller, so no client can get either one wrong.
    let public_access = caller.user_id.is_none();

    // The file and its chunks are written in one transaction. For an account it
    // also holds the quota lock from the usage check to the commit, and the
    // chunks it writes are what reserve this upload's room for the next check.
    let mut tx = match axum_state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    let file = match &caller.user_id {
        // linked (sanctum) uploads
        Some(user_id) => {
            // Require a verified email before a registered user can upload.
            if !caller.email_verified {
                return respond(
                    403,
                    "Email not verified",
                    vec!["Please verify your email address before uploading.".to_string()],
                    json!({ "email_verification_required": true }),
                );
            }

            // Enforce the account's storage quota before accepting the upload,
            // counting the uploads it has started but not finished.
            if libs::quota::lock(&mut tx, user_id).await.is_err() {
                return respond(500, "Database error", vec![], json!({}));
            }
            if let Some(q) = libs::quota::for_user(&mut tx, user_id).await {
                if q.would_exceed(libs::uploads::stored_total(&chunk_sizes, payload.file_encrypted)) {
                    return respond(
                        413,
                        "Storage quota exceeded",
                        vec!["This upload would exceed your storage limit. Free up space or upgrade your plan.".to_string()],
                        json!({ "limit": q.limit, "used": q.used }),
                    );
                }
            }

            let insert_file_query = sqlx::query_as!(
                models::File,
                "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, sha256_checksum, blake3_checksum, public_access, folder_id, owner_api_key)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *",
                user_id,
                payload.file_name,
                payload.file_mime,
                payload.file_size,
                payload.file_encrypted,
                payload.chunks.len() as i64,
                payload.sha256_checksum,
                payload.blake3_checksum,
                public_access,
                payload.folder_id,
                owner_api_key
            ).fetch_one(&mut *tx).await;

            match insert_file_query {
                Ok(file) => file,
                Err(_e) => {
                    return respond(
                        500,
                        "Failed to create upload",
                        vec![],
                        json!({}),
                    );
                }
            }
        }
        // anonymous (shadow) uploads
        None => {
            // Enforce bandwidth limit for the shadow key's IP.
            let user_info = sqlx::query!(
                "SELECT ip_address FROM anonymous_users WHERE api_key = $1",
                owner_api_key
            )
            .fetch_optional(&axum_state.pg_pool)
            .await;

            if let Ok(Some(info)) = user_info {
                let limit: i64 = 50 * 1024 * 1024 * 1024; // 50 GB

                // Reset stats for all keys on this IP older than 24h.
                let _ = sqlx::query!(
                    "UPDATE anonymous_users
                     SET bandwidth_usage_bytes = 0, last_reset_stats = NOW()
                     WHERE ip_address = $1 AND last_reset_stats < NOW() - INTERVAL '24 hours'",
                    info.ip_address
                )
                .execute(&axum_state.pg_pool)
                .await;

                // Sum total usage for this IP.
                let ip_stats = sqlx::query!(
                    "SELECT COALESCE(SUM(bandwidth_usage_bytes), 0)::BIGINT as total_usage FROM anonymous_users WHERE ip_address = $1",
                    info.ip_address
                )
                .fetch_one(&axum_state.pg_pool)
                .await;

                if let Ok(stats) = ip_stats {
                    let current_usage = stats.total_usage.unwrap_or(0);
                    if current_usage + payload.file_size > limit {
                        return respond(
                            403,
                            "Daily upload limit reached (50GB)",
                            vec!["Bandwidth limit for your IP exceeded. Please try again later or upgrade.".to_string()],
                            json!({}),
                        );
                    }
                }
            }

            let insert_file_query = sqlx::query_as!(
                models::File,
                "INSERT INTO files (name, mime, size, encrypted, total_chunks, sha256_checksum, blake3_checksum, public_access, folder_id, owner_api_key)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
                payload.file_name,
                payload.file_mime,
                payload.file_size,
                payload.file_encrypted,
                payload.chunks.len() as i64,
                payload.sha256_checksum,
                payload.blake3_checksum,
                public_access,
                payload.folder_id,
                owner_api_key
            ).fetch_one(&mut *tx).await;

            match insert_file_query {
                Ok(file) => {
                    // Update bandwidth usage for the shadow key.
                    if let Some(ref api_key) = file.owner_api_key {
                        let user_stats = sqlx::query!(
                            "SELECT bandwidth_usage_bytes, last_reset_stats FROM anonymous_users WHERE api_key = $1",
                            api_key
                        )
                        .fetch_optional(&axum_state.pg_pool)
                        .await;

                        if let Ok(Some(stats)) = user_stats {
                            let now = chrono::Utc::now();
                            let (new_usage, should_reset) = if now.signed_duration_since(stats.last_reset_stats).num_hours() >= 24 {
                                (file.size, true)
                            } else {
                                (stats.bandwidth_usage_bytes + file.size, false)
                            };

                            if should_reset {
                                let _ = sqlx::query!(
                                    "UPDATE anonymous_users SET bandwidth_usage_bytes = $1, last_reset_stats = $2 WHERE api_key = $3",
                                    new_usage,
                                    now,
                                    api_key
                                )
                                .execute(&axum_state.pg_pool)
                                .await;
                            } else {
                                let _ = sqlx::query!(
                                    "UPDATE anonymous_users SET bandwidth_usage_bytes = $1 WHERE api_key = $2",
                                    new_usage,
                                    api_key
                                )
                                .execute(&axum_state.pg_pool)
                                .await;
                            }
                        }
                    }
                    file
                }
                Err(_e) => {
                    return respond(
                        500,
                        "Failed to create upload",
                        vec![],
                        json!({}),
                    );
                }
            }
        }
    };

    // create chunks for the file
    let mut chunk_index = 0;
    let mut chunks = Vec::new();
    for chunk in payload.chunks {

        let chunk_id = format!("{}-{}", &crate::libs::rng::uuid(), &crate::libs::rng::uuid());

        let presigned_url = match axum_state.r2.presigned_put_url(
            if file.user_id.is_some() { "sanctum" } else { "shadow" },
            &chunk_id,
            libs::uploads::stored_len(chunk.size, payload.file_encrypted),
        ).await {
            Ok(url) => url,
            Err(_e) => {
                return respond(
                    500,
                    "Failed to generate presigned URL",
                    vec![],
                    json!({}),
                );
            }
        };

        let insert_chunk_query = sqlx::query_as::<_, models::Chunk>(
            "INSERT INTO chunks (id, file_id, chunk_index, size, presigned_url, file_offset, size_on_server, checksum, salt, nonce)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *"
        )
        .bind(chunk_id)
        .bind(file.id.clone())
        .bind(chunk_index)
        .bind(chunk.size)
        .bind(presigned_url)
        .bind(chunk.start)
        .bind(0)
        .bind(chunk.checksum)
        .bind(chunk.salt)
        .bind(chunk.nonce)
        .fetch_one(&mut *tx).await;

        match insert_chunk_query {
            Ok(chunk) => chunks.push(chunk),
            Err(_e) => {
                return respond(
                    500,
                    "Failed to create file chunk",
                    vec![],
                    json!({}),
                );
            }
        }

        chunk_index += 1;

    }

    if tx.commit().await.is_err() {
        return respond(500, "Failed to create upload", vec![], json!({}));
    }

    respond(
        200,
        "File upload started",
        vec![],
        json!({"file": file, "chunks": chunks}),
    )
}

#[cfg(test)]
mod tests {
    //! These call the handler against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Each deletes what it
    //! made.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::http::StatusCode;

    const MIB: i64 = 1024 * 1024;

    async fn create(st: &crate::AppState, caller: Caller, body: serde_json::Value) -> (StatusCode, serde_json::Value) {
        let payload: PayloadBody = serde_json::from_value(body).unwrap();
        let resp = handle(
            ConnectInfo("127.0.0.1:1".parse().unwrap()),
            State(st.clone()),
            Extension(Some(caller)),
            HeaderMap::new(),
            Json(payload),
        )
        .await
        .into_response();
        let status = resp.status();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap_or(json!({})))
    }

    fn body(file_size: i64, encrypted: bool, sizes: &[i64]) -> serde_json::Value {
        let mut start = 0i64;
        let chunks: Vec<serde_json::Value> = sizes
            .iter()
            .map(|s| {
                let c = json!({ "start": start, "end": start.wrapping_add(*s), "size": s, "checksum": "pending" });
                start = start.wrapping_add(*s);
                c
            })
            .collect();
        json!({
            "file_encrypted": encrypted,
            "file_name": "f.bin",
            "file_mime": "application/octet-stream",
            "file_size": file_size,
            "chunks": chunks,
            "sha256_checksum": "s",
            "blake3_checksum": ""
        })
    }

    async fn files_of(st: &crate::AppState, user: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE user_id = $1")
            .bind(user)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn as_user(st: &crate::AppState, file_size: i64, encrypted: bool, sizes: &[i64]) -> (StatusCode, serde_json::Value, i64) {
        let (user, key) = account(&st.pg_pool).await;
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let (status, v) = create(st, caller, body(file_size, encrypted, sizes)).await;
        let made = files_of(st, &user).await;
        forget(&st.pg_pool, &user).await;
        (status, v, made)
    }

    #[tokio::test]
    async fn a_negative_chunk_size_is_refused() {
        // Admitted on file_size 0, then marked complete without an upload, this
        // chunk used to drive the account's counted usage near -2^62.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, 0, false, &[-4611686018427387904]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_negative_file_size_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, -5 * MIB, false, &[]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn chunks_that_do_not_add_up_to_the_file_are_refused() {
        // Declaring 1 byte admits the upload; the chunk then claims 50 MiB.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, 1, false, &[50 * MIB]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_chunk_larger_than_any_client_sends_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, 300 * MIB, false, &[300 * MIB]).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn a_flood_of_tiny_chunks_is_refused() {
        // Each chunk is a presigned PUT and, if abandoned, a delete for the
        // orphan sweep to make. 5,000 one-byte chunks is not a real upload.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let sizes = vec![1i64; 5000];
        let (status, _, made) = as_user(&st, 5000, false, &sizes).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn an_anonymous_drop_with_a_negative_size_is_refused() {
        // A negative file_size used to lower the IP's recorded daily usage.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let key = format!("shadow-{}", crate::libs::rng::uuid());
        let caller = Caller { api_key: key.clone(), user_id: None, email_verified: true };
        let (status, _) = create(&st, caller, body(-20 * 1024 * MIB, false, &[])).await;
        let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE owner_api_key = $1")
            .bind(&key)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        let _ = sqlx::query("DELETE FROM files WHERE owner_api_key = $1").bind(&key).execute(&st.pg_pool).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    fn query_param(url: &str, name: &str) -> Option<String> {
        let q = url.split_once('?')?.1;
        q.split('&').find_map(|kv| {
            let (k, v) = kv.split_once('=')?;
            (k == name).then(|| v.replace("%3B", ";").replace("%3b", ";"))
        })
    }

    #[tokio::test]
    async fn each_upload_url_is_bound_to_its_chunk_length() {
        // The web app's shape: 16 MiB chunks, encrypted, so each PUT carries the
        // plaintext length plus a 16-byte tag. Unbound, one URL took up to 5 GiB.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, v, made) = as_user(&st, 16 * MIB + 5, true, &[16 * MIB, 5]).await;
        assert_eq!(status, StatusCode::OK, "{v}");
        assert_eq!(made, 1);
        let chunks = v.pointer("/data/chunks").and_then(|c| c.as_array()).cloned().unwrap();
        assert_eq!(chunks.len(), 2);
        for c in &chunks {
            let url = c["presigned_url"].as_str().unwrap();
            let signed = query_param(url, "X-Amz-SignedHeaders").unwrap();
            assert!(signed.split(';').any(|h| h == "content-length"), "length not signed: {signed}");
            let expires: u64 = query_param(url, "X-Amz-Expires").unwrap().parse().unwrap();
            assert!(expires <= 2 * 60 * 60, "PUT url lives {expires}s");
        }
    }

    #[tokio::test]
    async fn an_empty_file_with_no_chunks_is_still_accepted() {
        // The drive page sends an empty file as zero chunks.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, 0, false, &[]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(made, 1);
    }

    /// A verified account on a plan of `limit` bytes, and the caller for it.
    async fn on_plan(st: &crate::AppState, limit: i64) -> (String, Caller) {
        let (user, key) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = $2 WHERE id = $1")
            .bind(&user)
            .bind(limit)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        (user.clone(), Caller { api_key: key, user_id: Some(user), email_verified: true })
    }

    #[tokio::test]
    async fn uploads_started_before_any_is_stored_share_the_plan() {
        // Nothing is stored until the PUTs land, so five creates in a row each
        // used to be checked against an empty account and all were admitted.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, caller) = on_plan(&st, 100).await;
        let mut statuses = Vec::new();
        for _ in 0..5 {
            statuses.push(create(&st, caller.clone(), body(80, false, &[80])).await.0);
        }
        let made = files_of(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(statuses[0], StatusCode::OK);
        assert!(statuses[1..].iter().all(|s| *s == StatusCode::PAYLOAD_TOO_LARGE), "{statuses:?}");
        assert_eq!(made, 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn uploads_started_at_once_share_the_plan() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, caller) = on_plan(&st, 100).await;
        let tasks: Vec<_> = (0..8)
            .map(|_| {
                let (st, caller) = (st.clone(), caller.clone());
                tokio::spawn(async move { create(&st, caller, body(80, false, &[80])).await.0 })
            })
            .collect();
        let mut statuses = Vec::new();
        for t in tasks {
            statuses.push(t.await.unwrap());
        }
        let made = files_of(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(statuses.iter().filter(|s| **s == StatusCode::OK).count(), 1, "{statuses:?}");
        assert_eq!(made, 1);
    }

    #[tokio::test]
    async fn trashing_an_upload_in_flight_does_not_free_its_room() {
        // Its PUT URLs still work after it is trashed.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, caller) = on_plan(&st, 100).await;
        let first = create(&st, caller.clone(), body(80, false, &[80])).await.0;
        sqlx::query("UPDATE files SET deleted = true, deleted_on = NOW() WHERE user_id = $1")
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let second = create(&st, caller, body(80, false, &[80])).await.0;
        forget(&st.pg_pool, &user).await;
        assert_eq!(first, StatusCode::OK);
        assert_eq!(second, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn an_upload_whose_urls_have_expired_holds_no_room() {
        // Abandoned before the sweep reaches it: its URLs no longer work, so it
        // must not keep the account full.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, caller) = on_plan(&st, 100).await;
        let first = create(&st, caller.clone(), body(80, false, &[80])).await.0;
        sqlx::query("UPDATE chunks SET created_on = NOW() - INTERVAL '3 hours' WHERE file_id IN (SELECT id FROM files WHERE user_id = $1)")
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let second = create(&st, caller, body(80, false, &[80])).await.0;
        forget(&st.pg_pool, &user).await;
        assert_eq!(first, StatusCode::OK);
        assert_eq!(second, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_client_library_chunk_size_is_still_accepted() {
        // packages/client and the API docs use 100 MB chunks.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (status, _, made) = as_user(&st, 150 * MIB, true, &[100 * MIB, 50 * MIB]).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(made, 1);
    }

    /// Names a browser zip would turn into an entry outside the folder the
    /// recipient extracts into, or that no real file can have.
    const UNSAFE_NAMES: [&str; 8] = [
        "../../../AppData/Roaming/Microsoft/Windows/Start Menu/Programs/Startup/update.bat",
        "..",
        ".",
        "...",
        "a/b.txt",
        "..\\..\\evil.bat",
        "nul\0byte",
        "line\nbreak",
    ];

    #[tokio::test]
    async fn a_file_name_that_is_a_path_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let mut accepted = vec![];
        for name in UNSAFE_NAMES {
            let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
            let mut b = body(5, false, &[5]);
            b["file_name"] = json!(name);
            let (status, _) = create(&st, caller, b).await;
            if status != StatusCode::BAD_REQUEST {
                accepted.push((name, status));
            }
        }
        let made = files_of(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert!(accepted.is_empty(), "accepted: {accepted:?}");
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn an_anonymous_drop_named_as_a_path_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let key = format!("shadow-{}", crate::libs::rng::uuid());
        let caller = Caller { api_key: key.clone(), user_id: None, email_verified: true };
        let mut b = body(5, false, &[5]);
        b["file_name"] = json!("../../../Startup/update.bat");
        let (status, _) = create(&st, caller, b).await;
        let made: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE owner_api_key = $1")
            .bind(&key)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        let _ = sqlx::query("DELETE FROM chunks WHERE file_id IN (SELECT id FROM files WHERE owner_api_key = $1)")
            .bind(&key)
            .execute(&st.pg_pool)
            .await;
        let _ = sqlx::query("DELETE FROM files WHERE owner_api_key = $1").bind(&key).execute(&st.pg_pool).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(made, 0);
    }

    #[tokio::test]
    async fn ordinary_file_names_are_still_accepted() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let mut refused = vec![];
        for name in [".env", "..hidden", "notes (v2).final.txt", "Report: Q3 #1 é 名前.pdf", "a..b", "x"] {
            let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
            let mut b = body(5, false, &[5]);
            b["file_name"] = json!(name);
            let (status, _) = create(&st, caller, b).await;
            if status != StatusCode::OK {
                refused.push((name, status));
            }
        }
        forget(&st.pg_pool, &user).await;
        assert!(refused.is_empty(), "refused: {refused:?}");
    }
}
