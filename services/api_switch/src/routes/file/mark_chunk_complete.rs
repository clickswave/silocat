use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize)]
pub struct PayloadBody {
    chunk_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
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

    // 0. Resolve the chunk's owning file and verify the caller owns it before
    //    touching any upload state (no marking another caller's chunks complete).
    let owner = sqlx::query!(
        "SELECT f.id as file_id, f.user_id, f.owner_api_key
         FROM chunks c JOIN files f ON f.id = c.file_id
         WHERE c.id = $1",
        payload.chunk_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    let (file_id, file_user_id) = match owner {
        Ok(Some(rec)) => {
            if !caller.owns(&rec.user_id, &rec.owner_api_key) {
                return respond(404, "Chunk not found", vec![], json!({}));
            }
            (rec.file_id, rec.user_id)
        }
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    // 1. Mark the chunk uploaded only once HeadObject has measured its object
    //    at exactly the length its PUT URL was signed for, and record that
    //    measured size for quota. This used to fall back to the client-declared
    //    size whenever HeadObject failed, so a chunk nobody uploaded counted as
    //    whatever the client said, including a negative size that drove the
    //    account's usage below zero. A failed measurement is now retryable.
    let storage = if file_user_id.is_some() { "sanctum" } else { "shadow" };
    let expected = match crate::libs::uploads::chunk_expected_len(&axum_state.pg_pool, &payload.chunk_id).await {
        Ok(Some(n)) => n,
        Ok(None) => return respond(404, "Chunk not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };
    let sz = match crate::libs::uploads::measure_chunk(&axum_state.r2, storage, &payload.chunk_id, expected).await {
        crate::libs::uploads::Measured::Stored(sz) => sz,
        crate::libs::uploads::Measured::WrongSize => {
            return respond(400, "Chunk does not match its declared size", vec![], json!({}))
        }
        crate::libs::uploads::Measured::Unconfirmed => {
            return respond(503, "Could not confirm the chunk was stored. Try again.", vec![], json!({}))
        }
    };
    let upd = sqlx::query!(
        "UPDATE chunks SET uploaded = true, uploading = false, size_on_server = $1 WHERE id = $2",
        sz, payload.chunk_id
    )
    .execute(&axum_state.pg_pool)
    .await;
    if upd.is_err() {
        return respond(500, "Failed to update chunk status", vec![], json!({}));
    }

    // 2. Recount completed chunks and update the file.
    let count_query = sqlx::query!(
        "SELECT COUNT(*) as count FROM chunks WHERE file_id = $1 AND uploaded = true",
        file_id
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

    match count_query {
        Ok(record) => {
            let _ = sqlx::query!(
                "UPDATE files SET uploaded_chunks = $1 WHERE id = $2",
                record.count.unwrap_or(0) as i32,
                file_id
            )
            .execute(&axum_state.pg_pool)
            .await;
        }
        Err(e) => {
            println!("Failed to update file chunk count: {}", e);
        }
    }

    respond(200, "Chunk marked as complete", vec![], json!({}))
}

#[cfg(test)]
mod tests {
    //! These call the handler against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. The test storage
    //! client points at a closed port, so every HeadObject fails, which is the
    //! state of a chunk that was never uploaded.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::http::StatusCode;

    /// A one-chunk file owned by `user` whose chunk claims `size` bytes.
    async fn pending_chunk(pool: &sqlx::PgPool, user: &str, size: i64) -> String {
        let file_id: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, sha256_checksum, blake3_checksum, public_access) \
             VALUES ($1, 'f.bin', 'application/octet-stream', $2, false, 1, 's', '', false) RETURNING id",
        )
        .bind(user)
        .bind(size)
        .fetch_one(pool)
        .await
        .expect("seed file");
        let chunk_id = format!("{}-{}", crate::libs::rng::uuid(), crate::libs::rng::uuid());
        sqlx::query(
            "INSERT INTO chunks (id, file_id, chunk_index, size, presigned_url, file_offset, size_on_server, checksum) \
             VALUES ($1, $2, 0, $3, '', 0, 0, 'pending')",
        )
        .bind(&chunk_id)
        .bind(&file_id)
        .bind(size)
        .execute(pool)
        .await
        .expect("seed chunk");
        chunk_id
    }

    #[tokio::test]
    async fn a_chunk_that_was_never_stored_is_not_counted_at_its_declared_size() {
        // HeadObject failing used to fall back to the declared size, so a chunk
        // nobody uploaded counted as whatever the client said it was.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let chunk_id = pending_chunk(&st.pg_pool, &user, 5 * 1024 * 1024).await;
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let payload: PayloadBody = serde_json::from_value(json!({ "chunk_id": chunk_id })).unwrap();
        let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload)).await.into_response().status();
        let (uploaded, on_server): (bool, i64) =
            sqlx::query_as("SELECT uploaded, size_on_server FROM chunks WHERE id = $1")
                .bind(&chunk_id)
                .fetch_one(&st.pg_pool)
                .await
                .unwrap();
        forget(&st.pg_pool, &user).await;
        assert_ne!(status, StatusCode::OK);
        assert!(!uploaded);
        assert_eq!(on_server, 0);
    }
}
