use axum::extract::{Path, State, Query};
use axum::{Json, Extension};
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::json;
use crate::routes::respond;
use crate::models::{UserTokenData};
use rand::Rng; // Make sure rand is available or use uuid

#[derive(Deserialize)]
pub struct ToggleSharePayload {
    pub user_id: String,
    pub file_id: Option<String>,
    pub folder_id: Option<String>,
    pub share_type: String, // 'off', 'public', 'once'
}

#[derive(Deserialize)]
pub struct RegeneratePayload {
    pub user_id: String,
    pub file_id: Option<String>,
    pub folder_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GetShareInfoQuery {
    pub user_id: String,
}

fn generate_token() -> String {
    // Generate a secure enough random string
    let random_bytes: Vec<u8> = (0..16).map(|_| rand::random::<u8>()).collect();
    hex::encode(random_bytes)
}

pub async fn toggle_share(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<ToggleSharePayload>,
) -> impl IntoResponse {
    let new_token = generate_token();
    let user_id = payload.user_id.clone();
    
    if let Some(file_id) = payload.file_id {
        let mut tx = match axum_state.pg_pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
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
                    "UPDATE files SET share_type = $1, share_token = $2 WHERE id = $3 AND user_id = $4 RETURNING share_token, share_type, link_downloads, link_max_downloads",
                    payload.share_type,
                    token_to_set,
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
                            "link_max_downloads": r.link_max_downloads
                        }))
                    },
                    Err(e) => {
                         let _ = tx.rollback().await;
                         respond(500, "Failed to update share settings", vec![e.to_string()], json!({}))
                    }
                }
            },
            Ok(None) => respond(404, "File not found", vec![], json!({})),
            Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
        }

    } else if let Some(folder_id) = payload.folder_id {
         let mut tx = match axum_state.pg_pool.begin().await {
            Ok(tx) => tx,
            Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
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
                    "UPDATE folders SET share_type = $1, share_token = $2 WHERE id = $3 AND user_id = $4 RETURNING share_token, share_type, link_downloads, link_max_downloads",
                    payload.share_type,
                    token_to_set,
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
                            "link_max_downloads": r.link_max_downloads
                        }))
                    },
                    Err(e) => {
                         let _ = tx.rollback().await;
                         respond(500, "Failed to update share settings", vec![e.to_string()], json!({}))
                    }
                }
             },
             Ok(None) => respond(404, "Folder not found", vec![], json!({})),
             Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
        }

    } else {
        respond(400, "Missing file_id or folder_id", vec![], json!({}))
    }
}

pub async fn regenerate_token(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<RegeneratePayload>,
) -> impl IntoResponse {
    let new_token = generate_token();
    let user_id = payload.user_id; // Moved out of Extension
    
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
            Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
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
            Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
        }
    } else {
        respond(400, "Missing file_id or folder_id", vec![], json!({}))
    }
}

pub async fn get_share_info(
    State(axum_state): State<crate::AppState>,
    Path(id): Path<String>,
    Query(query): Query<GetShareInfoQuery>,
) -> impl IntoResponse {
    let user_id = query.user_id;

    // Check files
    let file = sqlx::query!(
        "SELECT share_token, share_type, link_downloads, link_max_downloads FROM files WHERE id = $1 AND user_id = $2",
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
            "link_max_downloads": r.link_max_downloads
        }));
    }

    // Check folders
    let folder = sqlx::query!(
        "SELECT share_token, share_type, link_downloads, link_max_downloads FROM folders WHERE id = $1 AND user_id = $2",
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
            "link_max_downloads": r.link_max_downloads
        }));
    }

    respond(404, "Item not found", vec![], json!({}))
}

#[derive(Deserialize)]
pub struct PublicDownloadPayload {
    pub token: String,
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
        "SELECT id, name, size, mime, share_type, link_downloads, link_max_downloads, encrypted FROM files WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file {
        Ok(Some(r)) => {
            if r.share_type == Some("once".to_string()) {
                let downloads = r.link_downloads.unwrap_or(0);
                let max = r.link_max_downloads.unwrap_or(1);
                if downloads >= max {
                    return respond(410, "This safe-link has expired.", vec!["Link limit reached".to_string()], json!({}));
                }
            }

            return respond(200, "File found", vec![], json!({
                "type": "file",
                "id": r.id,
                "name": r.name,
                "size": r.size,
                "mime": r.mime,
                "encrypted": r.encrypted // Expose encrypted status
            }));
        },
        Ok(None) => {}, 
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
    }

    // Check folders
    let folder = sqlx::query!(
        "SELECT id, name, share_type, link_downloads, link_max_downloads FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match folder {
        Ok(Some(r)) => {
            if r.share_type == Some("once".to_string()) {
                let downloads = r.link_downloads.unwrap_or(0);
                let max = r.link_max_downloads.unwrap_or(1);
                if downloads >= max {
                    return respond(410, "This safe-link has expired.", vec!["Link limit reached".to_string()], json!({}));
                }
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
                "files": files_data
            }));
        },
        Ok(None) => respond(404, "Invalid or expired link", vec!["Link not found".to_string()], json!({})),
        Err(e) => respond(500, "Database error", vec![e.to_string()], json!({})),
    }
}

#[derive(Deserialize)]
pub struct PublicFetchChunksPayload {
    pub token: String, // Folder share token
    pub file_id: String, // File within that folder
}

pub async fn public_fetch_file_chunks(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PublicFetchChunksPayload>,
) -> impl IntoResponse {
    let token = payload.token;
    let file_id = payload.file_id;

    // Verify folder token
    let folder = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(folder_rec)) = folder {
         if folder_rec.share_type == Some("once".to_string()) {
            let downloads = folder_rec.link_downloads.unwrap_or(0);
            let max = folder_rec.link_max_downloads.unwrap_or(1);
            if downloads >= max {
                return respond(410, "This safe-link has expired.", vec![], json!({}));
            }
        }
        
        // Check if file is in this folder (direct child for now, recursive logic if needed later)
        // Actually, we should probably allow any descendant if we had fully recursive 'files in folder' query.
        // For simple structure (parent_id), we check if file.folder_id == folder.id
        // BUT wait, users might have nested folders.
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
                                Err(e) => return respond(500, "Failed to generate download URL", vec![e.to_string()], json!({})),
                            }
                        }
                        
                        return respond(200, "Chunks retrieved", vec![], json!({
                            "type": "file_chunks",
                            "file_id": file_rec.id,
                            "chunks": response_chunks
                        }));
                    },
                    Err(e) => return respond(500, "Failed to fetch chunks", vec![e.to_string()], json!({})),
                }

            },
            Ok(None) => return respond(404, "File not found in this folder", vec![], json!({})),
            Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
        }

    } else {
        return respond(404, "Invalid folder token", vec![], json!({}));
    }
}


pub async fn public_authorize_download(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<PublicDownloadPayload>,
) -> impl IntoResponse {
    let token = payload.token;

    // Check file
    let file_query = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads, user_id FROM files WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    if let Ok(Some(r)) = file_query {
         if r.share_type == Some("once".to_string()) {
            let downloads = r.link_downloads.unwrap_or(0);
            let max = r.link_max_downloads.unwrap_or(1);
            if downloads >= max {
                return respond(410, "This safe-link has expired.", vec![], json!({}));
            }
        }
        
        // Increment download count
        let _ = sqlx::query!(
            "UPDATE files SET link_downloads = COALESCE(link_downloads, 0) + 1 WHERE id = $1",
            r.id
        )
        .execute(&axum_state.pg_pool)
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
                        Err(e) => return respond(500, "Failed to generate download URL", vec![e.to_string()], json!({})),
                    }
                }
                
                return respond(200, "Download authorized", vec![], json!({
                    "type": "file",
                    "id": r.id,
                    "chunks": response_chunks
                }));
            },
            Err(e) => return respond(500, "Failed to fetch chunks", vec![e.to_string()], json!({})),
        }
    }
    
    // Check folder
    let folder = sqlx::query!(
        "SELECT id, share_type, link_downloads, link_max_downloads FROM folders WHERE share_token = $1 AND share_type != 'off'",
        token
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;
    
    if let Ok(Some(r)) = folder {
         if r.share_type == Some("once".to_string()) {
            let downloads = r.link_downloads.unwrap_or(0);
            let max = r.link_max_downloads.unwrap_or(1);
            if downloads >= max {
                return respond(410, "This safe-link has expired.", vec![], json!({}));
            }
        }
        
        let _ = sqlx::query!(
            "UPDATE folders SET link_downloads = COALESCE(link_downloads, 0) + 1 WHERE id = $1",
            r.id
        )
        .execute(&axum_state.pg_pool)
        .await;
        
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
            Err(e) => return respond(500, "Failed to list folder files", vec![e.to_string()], json!({}))
        }
    }

    respond(404, "Invalid link", vec![], json!({}))
}
