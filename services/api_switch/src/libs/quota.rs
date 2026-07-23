//! Per-user storage quota checks.
//!
//! Authenticated uploads previously issued presigned PUT URLs with no quota
//! check, so any account could upload past its plan. `for_user` mirrors the
//! limit/used computation used by the storage-stats endpoint, and admission is
//! decided before an upload is accepted.
//!
//! NOTE: `used` and the admitted size are summed from the client-declared
//! `files.size`. A malicious client can under-declare; the complete fix is to
//! record each chunk's real R2 object size (HeadObject) on mark-chunk-complete
//! and compute usage from that. This check enforces the plan for honest clients
//! and is the primary business-model guard.

use sqlx::{Pool, Postgres};

pub struct Quota {
    pub limit: i64,
    pub used: i64,
}

impl Quota {
    pub fn would_exceed(&self, additional: i64) -> bool {
        self.used.saturating_add(additional) > self.limit
    }
}

/// Compute a user's storage limit (base + active non-expired subscription space)
/// and current used bytes (completed, non-deleted files). Returns `None` on a DB
/// error or unknown user.
pub async fn for_user(pool: &Pool<Postgres>, user_id: &str) -> Option<Quota> {
    let limit = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT ((SELECT default_storage_bytes FROM users WHERE id = $1) \
              + COALESCE((SELECT SUM(additional_space) FROM subscriptions \
                          WHERE created_by = $1 AND expires_on > NOW()), 0))::BIGINT",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .ok()
    .flatten()?;

    let used = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(size), 0)::BIGINT FROM files \
         WHERE user_id = $1 AND deleted = false AND uploaded_chunks >= total_chunks",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    Some(Quota { limit, used })
}
