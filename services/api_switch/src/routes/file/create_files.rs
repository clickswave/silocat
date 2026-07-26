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
    pub public_access: bool,
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

            // Enforce the account's storage quota before accepting the upload.
            if let Some(q) = libs::quota::for_user(&axum_state.pg_pool, user_id).await {
                if q.would_exceed(payload.file_size) {
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
                payload.public_access,
                payload.folder_id,
                owner_api_key
            ).fetch_one(&axum_state.pg_pool).await;

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
                payload.public_access,
                payload.folder_id,
                owner_api_key
            ).fetch_one(&axum_state.pg_pool).await;

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
            &chunk_id
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
        .fetch_one(&axum_state.pg_pool).await;

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

    respond(
        200,
        "File upload started",
        vec![],
        json!({"file": file, "chunks": chunks}),
    )
}
