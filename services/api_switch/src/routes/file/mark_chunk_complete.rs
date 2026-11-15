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
    match crate::libs::uploads::mark_stored(&axum_state.pg_pool, &payload.chunk_id, sz).await {
        Ok(crate::libs::uploads::Marked::Stored) => {}
        Ok(crate::libs::uploads::Marked::OverPlan(q)) => {
            return respond(
                413,
                "Storage quota exceeded",
                vec!["This upload took too long and its space has been given back. It no longer fits your storage limit.".to_string()],
                json!({ "limit": q.limit, "used": q.used }),
            )
        }
        Ok(crate::libs::uploads::Marked::Gone) => return respond(404, "Chunk not found", vec![], json!({})),
        Err(_e) => return respond(500, "Failed to update chunk status", vec![], json!({})),
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

    /// An account on a plan of `limit` bytes already holding `live` stored
    /// bytes, with a pending 80-byte chunk created `age` ago in a file that is
    /// trashed or not. Marks that chunk complete against storage that holds it
    /// and returns the status and whether the chunk now counts as stored.
    async fn mark_pending(limit: i64, live: i64, age: &str, trashed: bool) -> Option<(StatusCode, bool)> {
        let mut st = state().await?;
        st.r2 = crate::test_support::storage_holding(80);
        let (user, key) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = $2 WHERE id = $1")
            .bind(&user)
            .bind(limit)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        if live > 0 {
            let stored = pending_chunk(&st.pg_pool, &user, live).await;
            sqlx::query("UPDATE chunks SET uploaded = true, size_on_server = size WHERE id = $1")
                .bind(&stored)
                .execute(&st.pg_pool)
                .await
                .unwrap();
        }
        let chunk_id = pending_chunk(&st.pg_pool, &user, 80).await;
        sqlx::query(&format!("UPDATE chunks SET created_on = NOW() - INTERVAL '{age}' WHERE id = $1"))
            .bind(&chunk_id)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        sqlx::query("UPDATE files SET deleted = $2, deleted_on = CASE WHEN $2 THEN NOW() END WHERE id = (SELECT file_id FROM chunks WHERE id = $1)")
            .bind(&chunk_id)
            .bind(trashed)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let payload: PayloadBody = serde_json::from_value(json!({ "chunk_id": chunk_id })).unwrap();
        let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload)).await.into_response().status();
        let uploaded: bool = sqlx::query_scalar("SELECT uploaded FROM chunks WHERE id = $1")
            .bind(&chunk_id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &user).await;
        Some((status, uploaded))
    }

    #[tokio::test]
    async fn a_chunk_marked_after_its_room_lapsed_cannot_pass_the_plan() {
        // Its room was given back when its PUT URL expired and another upload
        // took it. Marking it complete afterwards stored two plans' worth.
        let Some((status, uploaded)) = mark_pending(100, 80, "3 hours", false).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(!uploaded);
    }

    #[tokio::test]
    async fn a_late_chunk_that_fits_the_plan_is_counted() {
        let Some((status, uploaded)) = mark_pending(200, 80, "3 hours", false).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::OK);
        assert!(uploaded);
    }

    #[tokio::test]
    async fn a_chunk_marked_in_time_uses_the_room_it_already_holds() {
        // Its room was counted from the moment it was created, so the plan
        // being full of it is no reason to refuse it.
        let Some((status, uploaded)) = mark_pending(100, 0, "1 minute", false).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::OK);
        assert!(uploaded);
    }

    #[tokio::test]
    async fn a_late_chunk_of_a_trashed_file_waits_for_restore_to_be_checked() {
        // Trashed bytes do not count, so marking it takes no room now.
        // Restoring the file is what has to fit it in the plan.
        let Some((status, uploaded)) = mark_pending(100, 80, "3 hours", true).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::OK);
        assert!(uploaded);
    }
}
