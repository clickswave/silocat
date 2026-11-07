//! When an account was last active. Read by the admin console; written here
//! from the auth middlewares.

/// Record that `user_id` just made an authenticated request.
///
/// Runs in the background so the request never waits on it, and only writes
/// when the stored time is more than five minutes old, so a busy session costs
/// one UPDATE per five minutes rather than one per request. A failure is
/// dropped: last-active is a reporting aid, never a reason to fail a request.
pub fn touch(pool: &sqlx::PgPool, user_id: &str) {
    let pool = pool.clone();
    let id = user_id.to_string();
    tokio::spawn(async move {
        let _ = sqlx::query(
            "UPDATE users SET last_active_at = NOW() \
             WHERE id = $1 AND (last_active_at IS NULL OR last_active_at < NOW() - INTERVAL '5 minutes')",
        )
        .bind(&id)
        .execute(&pool)
        .await;
    });
}
