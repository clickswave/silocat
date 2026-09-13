use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::Extension;
use axum::Json;
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    pub file_id: String,
}

#[derive(serde::Serialize)]
pub struct ChunkResponse {
    pub id: String,
    pub index: i32,
    pub size: i64,
    pub presigned_url: String, // Download URL
    pub file_offset: i64,
    pub checksum: String,
    pub nonce: Option<String>,
    pub salt: Option<String>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {

    // 1. Get file info to determine storage bucket (shadow/sanctum)
    let file_query = sqlx::query_as!(
        models::File,
        "SELECT * FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;
    
    let file = match file_query {
        Ok(Some(f)) => f,
        Ok(None) => return respond(404, "File not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    // Access control. The owner always reaches their own chunks via `owns`. A
    // non-owner gets download URLs only via public_access AND only when the file
    // carries no active share protection (password / one-time / expiry / delete);
    // protected shares must go through the token-checked /file/public/share/*
    // path so those controls are actually enforced.
    let share_protected = crate::libs::sharing::share_protected(
        file.share_password_hash.as_deref(),
        file.share_type.as_deref(),
        file.share_expires_at,
        file.deleted,
    );
    let owns = caller
        .as_ref()
        .map_or(false, |c| c.owns(&file.user_id, &file.owner_api_key));

    // The folder holding this file has a say in both directions.
    //
    // It can grant: publishing a folder is a statement about its contents, so a
    // file inside one is downloadable without being published itself. A
    // registered user's files stay private individually (0044) and only the
    // folder carries the share, and the token path has always worked this way
    // (it authorizes any file in the shared folder without consulting the file's
    // own flag), so the two read paths now agree instead of disagreeing quietly.
    //
    // And it can refuse: a password or an expiry on a folder covers everything
    // under it. Otherwise the control is bypassed by asking for a file id
    // directly instead of the folder's, and the folder listing is where those
    // ids come from.
    let folder_access = match file.folder_id.as_deref() {
        Some(folder_id) => crate::libs::sharing::folder_access(&axum_state.pg_pool, folder_id).await,
        None => crate::libs::sharing::FolderAccess { published: false, protected: false },
    };

    let allowed = owns
        || (!share_protected
            && !folder_access.protected
            && (file.public_access || folder_access.published));

    if !allowed {
        return respond(404, "File not found", vec![], json!({}));
    }

    let storage_type = if file.user_id.is_some() { "sanctum" } else { "shadow" };

    // 2. Fetch all chunks
    let chunks_query = sqlx::query_as::<_, models::Chunk>(
        "SELECT * FROM chunks WHERE file_id = $1 AND uploaded = true ORDER BY chunk_index ASC",
    )
    .bind(payload.file_id)
    .fetch_all(&axum_state.pg_pool)
    .await;

    match chunks_query {
        Ok(chunks) => {
            let mut response_chunks = Vec::new();

            for chunk in chunks {
                // Generate presigned GET url
                match axum_state.r2.presigned_get_url(storage_type, &chunk.id).await {
                    Ok(url) => {
                        response_chunks.push(ChunkResponse {
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
                    Err(_e) => {
                        return respond(500, "Failed to generate download URL", vec![], json!({}));
                    }
                }
            }
            
            respond(
                200,
                "Chunks retrieved",
                vec![],
                json!({ "chunks": response_chunks }),
            )
        },
        Err(_e) => {
             respond(
                500,
                "Failed to fetch chunks",
                vec![],
                json!({}),
            )
        }
    }
}
