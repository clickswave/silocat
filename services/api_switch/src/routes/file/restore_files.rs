use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

#[derive(Deserialize)]
pub struct RestorePayload {
    pub file_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<RestorePayload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    // Shadow (anonymous) sessions may restore their own files, so any caller is allowed.
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

    // 1. Fetch file ownership info
    let file_query = sqlx::query!(
        "SELECT user_id, owner_api_key FROM files WHERE id = $1",
        payload.file_id
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    match file_query {
        Ok(Some(record)) => {
            // 2. Verify ownership. Return 404 (not 403) on non-ownership to avoid
            // leaking the existence of another caller's file.
            if !caller.owns(&record.user_id, &record.owner_api_key) {
                return respond(404, "File not found", vec![], json!({}));
            }

            // 3. Trashed files stop counting toward the plan while their bytes
            //    stay stored for the 30-day retention, so restoring one has to
            //    fit the plan again. Without this, uploading to the cap, trashing
            //    and repeating, then restoring everything held any amount. The
            //    quota lock is held from the check to the commit, so restores
            //    sent together cannot each fit into the same free space.
            let mut tx = match axum_state.pg_pool.begin().await {
                Ok(tx) => tx,
                Err(_e) => return respond(500, "Database error", vec![], json!({})),
            };
            if let Some(user_id) = &record.user_id {
                if crate::libs::quota::lock(&mut tx, user_id).await.is_err() {
                    return respond(500, "Database error", vec![], json!({}));
                }
                let trashed_bytes = sqlx::query_scalar::<_, Option<i64>>(
                    "SELECT COALESCE((SELECT SUM(GREATEST(c.size_on_server, 0)) FROM chunks c \
                                      WHERE c.file_id = f.id AND c.uploaded = true), 0)::BIGINT \
                     FROM files f WHERE f.id = $1 AND f.deleted = true",
                )
                .bind(&payload.file_id)
                .fetch_optional(&mut *tx)
                .await;
                let trashed_bytes = match trashed_bytes {
                    // Not in the trash: restoring it changes nothing.
                    Ok(None) => 0,
                    Ok(Some(n)) => n.unwrap_or(0),
                    Err(_e) => return respond(500, "Database error", vec![], json!({})),
                };
                if trashed_bytes > 0 {
                    match crate::libs::quota::for_user(&mut tx, user_id).await {
                        Some(q) if !q.would_exceed(trashed_bytes) => {}
                        Some(q) => {
                            return respond(
                                413,
                                "Storage quota exceeded",
                                vec!["Restoring this file would exceed your storage limit. Free up space or upgrade your plan.".to_string()],
                                json!({ "limit": q.limit, "used": q.used, "needed": trashed_bytes }),
                            )
                        }
                        None => return respond(500, "Database error", vec![], json!({})),
                    }
                }
            }

            // 4. Mark as not deleted
            let restore_result = sqlx::query!(
                "UPDATE files SET deleted = false, deleted_on = NULL WHERE id = $1",
                payload.file_id
            )
            .execute(&mut *tx)
            .await;

            match restore_result {
                Ok(_) if tx.commit().await.is_ok() => {
                    respond(200, "File restored successfully", vec![], json!({}))
                }
                _ => respond(500, "Failed to restore file", vec![], json!({})),
            }
        },
        Ok(None) => respond(404, "File not found", vec![], json!({})),
        Err(_e) => respond(500, "Database error", vec![], json!({})),
    }
}

#[cfg(test)]
mod tests {
    //! These call the handler against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::http::StatusCode;

    /// A file of `bytes` stored bytes owned by `user`, trashed or not.
    async fn stored_file(pool: &sqlx::PgPool, user: &str, bytes: i64, trashed: bool) -> String {
        let file_id: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, sha256_checksum, blake3_checksum, public_access, deleted, deleted_on) \
             VALUES ($1, 'f.bin', 'application/octet-stream', $2, false, 1, 1, 's', '', false, $3, CASE WHEN $3 THEN NOW() END) RETURNING id",
        )
        .bind(user)
        .bind(bytes)
        .bind(trashed)
        .fetch_one(pool)
        .await
        .expect("seed file");
        sqlx::query(
            "INSERT INTO chunks (id, file_id, chunk_index, size, presigned_url, file_offset, size_on_server, checksum, uploaded) \
             VALUES ($1, $2, 0, $3, '', 0, $3, 'c', true)",
        )
        .bind(crate::libs::rng::uuid())
        .bind(&file_id)
        .bind(bytes)
        .execute(pool)
        .await
        .expect("seed chunk");
        file_id
    }

    /// Restore a trashed file of `trashed` bytes for an account with a 100-byte
    /// plan already holding `live` bytes. Returns the status and whether the
    /// file is still in the trash afterwards.
    async fn restore(live: i64, trashed: i64) -> Option<(StatusCode, bool)> {
        let st = state().await?;
        let (user, key) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = 100 WHERE id = $1")
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        stored_file(&st.pg_pool, &user, live, false).await;
        let file_id = stored_file(&st.pg_pool, &user, trashed, true).await;
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let payload: RestorePayload = serde_json::from_value(json!({ "file_id": file_id })).unwrap();
        let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload)).await.into_response().status();
        let still_trashed: bool = sqlx::query_scalar("SELECT deleted FROM files WHERE id = $1")
            .bind(&file_id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &user).await;
        Some((status, still_trashed))
    }

    #[tokio::test]
    async fn a_trashed_file_cannot_be_restored_past_the_plan() {
        // Upload to the cap, trash it, upload again, then restore everything:
        // trashed bytes stop counting, so restore is where the plan is checked.
        let Some((status, still_trashed)) = restore(80, 80).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
        assert!(still_trashed);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 8)]
    async fn restores_sent_at_once_cannot_pass_the_plan() {
        // Each restore read usage and then flipped its file back with nothing
        // in between to stop the others doing the same.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = 100 WHERE id = $1")
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let mut ids = Vec::new();
        for _ in 0..8 {
            ids.push(stored_file(&st.pg_pool, &user, 80, true).await);
        }
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let tasks: Vec<_> = ids
            .into_iter()
            .map(|file_id| {
                let (st, caller) = (st.clone(), caller.clone());
                tokio::spawn(async move {
                    let payload: RestorePayload = serde_json::from_value(json!({ "file_id": file_id })).unwrap();
                    handle(State(st), Extension(Some(caller)), Json(payload)).await.into_response().status()
                })
            })
            .collect();
        let mut statuses = Vec::new();
        for t in tasks {
            statuses.push(t.await.unwrap());
        }
        let live: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM files WHERE user_id = $1 AND deleted = false")
            .bind(&user)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        forget(&st.pg_pool, &user).await;
        assert_eq!(live, 1, "{statuses:?}");
    }

    #[tokio::test]
    async fn an_upload_in_flight_keeps_its_room_from_a_restore() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = 100 WHERE id = $1")
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let pending = stored_file(&st.pg_pool, &user, 80, false).await;
        sqlx::query("UPDATE chunks SET uploaded = false, size_on_server = 0 WHERE file_id = $1")
            .bind(&pending)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let file_id = stored_file(&st.pg_pool, &user, 80, true).await;
        let caller = Caller { api_key: key, user_id: Some(user.clone()), email_verified: true };
        let payload: RestorePayload = serde_json::from_value(json!({ "file_id": file_id })).unwrap();
        let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload)).await.into_response().status();
        forget(&st.pg_pool, &user).await;
        assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn a_trashed_file_that_fits_is_restored() {
        let Some((status, still_trashed)) = restore(80, 20).await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        assert_eq!(status, StatusCode::OK);
        assert!(!still_trashed);
    }
}
