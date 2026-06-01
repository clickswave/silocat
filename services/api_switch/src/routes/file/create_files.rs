use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

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
pub struct File {
    user_id: Option<String>,
    file_encrypted: bool,
    file_name: String,
    file_mime: String,
    file_size: i64,
    chunks: Vec<ChunkData>,
    sha256_checksum: String,
    blake3_checksum: String,
    public_access: bool,
}

#[derive(serde::Deserialize, Debug)]
pub struct PayloadBody {
    pub storage_type: Option<String>, // shadow or sanctum
    pub user_id: Option<String>,
    
    pub file_encrypted: bool,
    pub file_name: String,
    pub file_mime: String,
    pub file_size: i64,
    pub chunks: Vec<ChunkData>,
    pub sha256_checksum: String,
    pub blake3_checksum: String,
    pub public_access: bool,
    pub folder_id: Option<String>,
    pub owner_api_key: Option<String>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    dbg!(&payload);

    let file = match payload.user_id {
        // linked uploads
        Some(user_id) => {
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
                payload.owner_api_key
            ).fetch_one(&axum_state.pg_pool).await;

            match insert_file_query {
                Ok(file) => file,
                Err(e) => {
                    // Handle error
                    return respond(
                        500,
                        "Failed to create upload",
                        vec![e.to_string()],
                        json!({}),
                    );
                }
            }
        }
        // anonymous uploads
        None => {
            // Enforce Bandwidth Limit for Shadow (Anonymous) Users
            if let Some(ref api_key) = payload.owner_api_key {
               // 1. Get IP and current stats for this key
               let user_info = sqlx::query!(
                   "SELECT ip_address FROM anonymous_users WHERE api_key = $1",
                   api_key
               )
               .fetch_optional(&axum_state.pg_pool)
               .await;

               if let Ok(Some(info)) = user_info {
                   let now = chrono::Utc::now();
                   let limit: i64 = 50 * 1024 * 1024 * 1024; // 50 GB

                   // 2. Reset stats for ALL keys associated with this IP if they are older than 24h
                   // This ensures expired usage from old keys doesn't count against the IP limit
                   let _ = sqlx::query!(
                       "UPDATE anonymous_users 
                        SET bandwidth_usage_bytes = 0, last_reset_stats = NOW() 
                        WHERE ip_address = $1 AND last_reset_stats < NOW() - INTERVAL '24 hours'",
                       info.ip_address
                   )
                   .execute(&axum_state.pg_pool)
                   .await;

                   // 3. Sum total usage for this IP
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
                payload.owner_api_key
            ).fetch_one(&axum_state.pg_pool).await;

            match insert_file_query {
                Ok(file) => {
                     // Update Bandwidth Usage
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
                },
                Err(e) => {
                    // Handle error
                    return respond(
                        500,
                        "Failed to create upload",
                        vec![e.to_string()],
                        json!({}),
                    );
                }
            }
        }
    };

    println!("222222222222222222222222222");
    // create chunks for the file
    let mut chunk_index = 0;
    let mut chunks = Vec::new();
    for chunk in payload.chunks {

        let chunk_id = format!("{}-{}",&crate::libs::rng::uuid(), &crate::libs::rng::uuid());

        let presigned_url = match axum_state.r2.presigned_put_url(
            if file.user_id.is_some() { "sanctum" } else { "shadow" },
            &chunk_id
        ).await {
            Ok(url) => url,
            Err(e) => {
                 return respond(
                    500,
                    "Failed to generate presigned URL",
                    vec![e.to_string()],
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
            Err(e) => {
                // Handle error
                return respond(
                    500,
                    "Failed to create file chunk",
                    vec![e.to_string()],
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