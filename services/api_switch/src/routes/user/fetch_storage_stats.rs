use axum::{extract::State, Extension, response::IntoResponse};
use serde::Serialize;
use serde_json::json;
use crate::{models::UserTokenData, routes::respond};

#[derive(Serialize, Debug)]
pub struct StorageStats {
    pub total: i64,
    pub used: i64,
    /// Room left under the plan, never below zero.
    pub free: i64,
    /// The account holds as much as its plan allows or more, as a lapsed
    /// subscription leaves it. It is read-only until it is under the plan:
    /// files stay readable and shareable, and nothing is deleted, but uploads
    /// and restores are refused.
    pub over_quota: bool,
}

// Identity comes from the authenticated X-Api-Key (validate_token), NEVER a body
// `user_id`. Previously this was a public route that returned any user's stats
// for an attacker-supplied `user_id`; now it can only report the caller's own.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
) -> impl IntoResponse {
    let user_id = user.id;

    // 1. Total storage limit = base (default_storage_bytes) + active, non-expired
    //    subscription space (promos / Pro grants live here, so they auto-expire).
    //    Runtime query (no macro) keeps the SQLX_OFFLINE prod build cache-free.
    let limit = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT ((SELECT default_storage_bytes FROM users WHERE id = $1) \
              + COALESCE((SELECT SUM(additional_space) FROM subscriptions \
                          WHERE created_by = $1 AND expires_on > NOW()), 0))::BIGINT"
    )
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await;

    let default_storage = match limit {
        Ok(Some(total)) => total,
        Ok(None) => return respond(404, "User not found", vec![], json!({})),
        Err(e) => {
            println!("Error fetching user storage limit: {:?}", e);
            return respond(500, "Database error", vec![], json!({}))
        }
    };

    // 2. Used storage = real stored bytes (each uploaded chunk's HeadObject
    //    size_on_server) for non-deleted files. Matches libs::quota so the UI
    //    counter and the upload gate agree, and can't be gamed by a client that
    //    under-declares files.size.
    let usage = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(GREATEST(c.size_on_server, 0)), 0)::BIGINT \
         FROM chunks c JOIN files f ON f.id = c.file_id \
         WHERE f.user_id = $1 AND f.deleted = false AND c.uploaded = true"
    )
    .bind(&user_id)
    .fetch_one(&state.pg_pool)
    .await;

    let used_bytes = match usage {
        Ok(v) => v.unwrap_or(0),
        Err(e) => {
             println!("Error calculating storage usage: {:?}", e);
             0 // Default to 0 on error for UI robustness
        }
    };

    let free_bytes = default_storage.saturating_sub(used_bytes).max(0);

    respond(
        200,
        "Storage stats fetched",
        vec![],
        json!(StorageStats {
            total: default_storage,
            used: used_bytes,
            free: free_bytes,
            over_quota: used_bytes >= default_storage,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::response::IntoResponse;

    fn caller(id: &str) -> UserTokenData {
        UserTokenData {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            profile_image: None,
            email_verified: true,
            password_set: true,
            username: id.to_string(),
            api_key: String::new(),
            account_type: "personal".to_string(),
            subscription: None,
            default_storage_bytes: 0,
            country: None,
            bio: None,
        }
    }

    /// An account on a base plan of `limit` bytes that has stored `stored` bytes.
    async fn holding(st: &crate::AppState, limit: i64, stored: i64) -> String {
        let (user, _) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET default_storage_bytes = $2 WHERE id = $1")
            .bind(&user)
            .bind(limit)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let file_id: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, sha256_checksum, blake3_checksum, public_access) \
             VALUES ($1, 'kept.bin', 'application/octet-stream', $2, false, 1, 1, 's', '', false) RETURNING id",
        )
        .bind(&user)
        .bind(stored)
        .fetch_one(&st.pg_pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO chunks (id, file_id, chunk_index, size, presigned_url, file_offset, size_on_server, checksum, uploaded) \
             VALUES ($1, $2, 0, $3, '', 0, $3, 'c', true)",
        )
        .bind(crate::libs::rng::uuid())
        .bind(&file_id)
        .bind(stored)
        .execute(&st.pg_pool)
        .await
        .unwrap();
        user
    }

    async fn stats(st: &crate::AppState, user: &str) -> serde_json::Value {
        let resp = handle(State(st.clone()), Extension(caller(user))).await.into_response();
        assert_eq!(resp.status().as_u16(), 200);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice::<serde_json::Value>(&body).unwrap()["data"].clone()
    }

    #[tokio::test]
    async fn an_account_over_its_plan_reports_no_free_space_and_says_so() {
        // A lapsed subscription leaves the account holding more than its base
        // plan. It keeps every file, read-only, and the stats say that rather
        // than a negative amount of free space.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let user = holding(&st, 100, 500).await;
        sqlx::query(
            "INSERT INTO subscriptions (id, name, additional_space, created_by, expires_on) \
             VALUES ($1, 'Plus', 1000, $2, NOW() - INTERVAL '1 day')",
        )
        .bind(crate::libs::rng::uuid())
        .bind(&user)
        .execute(&st.pg_pool)
        .await
        .unwrap();
        let v = stats(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(v["total"], 100, "{v}");
        assert_eq!(v["used"], 500, "{v}");
        assert_eq!(v["free"], 0, "{v}");
        assert_eq!(v["over_quota"], true, "{v}");
    }

    #[tokio::test]
    async fn a_full_account_has_no_room_left() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let user = holding(&st, 300, 300).await;
        let v = stats(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(v["free"], 0, "{v}");
        assert_eq!(v["over_quota"], true, "{v}");
    }

    #[tokio::test]
    async fn an_account_under_its_plan_reports_its_free_space() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let user = holding(&st, 1000, 400).await;
        let v = stats(&st, &user).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(v["total"], 1000, "{v}");
        assert_eq!(v["used"], 400, "{v}");
        assert_eq!(v["free"], 600, "{v}");
        assert_eq!(v["over_quota"], false, "{v}");
    }
}
