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

/// Bytes in a gibibyte. Every storage grant in the codebase is expressed as a
/// multiple of this rather than a literal byte count, because the two signup
/// paths once carried the same quantity as two different magic numbers.
pub const GIB: i64 = 1024 * 1024 * 1024;

/// What a new account gets when `SIGNUP_STORAGE_GB` is unset. This is the hosted
/// silo.cat free tier, so changing it changes the product.
const DEFAULT_SIGNUP_STORAGE_GIB: i64 = 10;

/// Sentinel for "no practical cap", used by self-hosted deployments where the
/// only real limit is the disk.
///
/// Deliberately not `i64::MAX`: `for_user` adds subscription space to this value
/// inside Postgres, and BIGINT addition at the type ceiling errors rather than
/// saturating. 1 PiB is unreachable on real hardware and leaves ~13 bits of
/// headroom for the addition.
const UNLIMITED_STORAGE_BYTES: i64 = 1024 * 1024 * GIB;

/// Storage granted to a newly created account, in bytes.
///
/// **Both signup paths must call this.** They previously hardcoded the same
/// concept as two different numbers, 10 GiB in `register_personal` and 50 GiB in
/// `google_auth`, so which plan you landed on depended on which button you
/// clicked. Neither was configurable, which meant a self-hoster pointing Silocat
/// at a 4 TB array still handed out 10 GiB accounts with no way to change it
/// short of recompiling.
///
/// `SIGNUP_STORAGE_GB` overrides the default. `0` or `unlimited` means no
/// practical cap; the self-host compose file sets it that way. An unparseable or
/// non-positive value falls back to the default rather than failing the signup,
/// since a bad env var should not take registration down.
pub fn signup_storage_bytes() -> i64 {
    let raw = std::env::var("SIGNUP_STORAGE_GB").ok();

    match raw.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
        None => DEFAULT_SIGNUP_STORAGE_GIB * GIB,
        Some(v) if v == "0" || v.eq_ignore_ascii_case("unlimited") => UNLIMITED_STORAGE_BYTES,
        Some(v) => match v.parse::<i64>() {
            Ok(gib) if gib > 0 => gib.saturating_mul(GIB).min(UNLIMITED_STORAGE_BYTES),
            _ => {
                eprintln!(
                    "[quota] SIGNUP_STORAGE_GB={:?} is not a positive integer; \
                     falling back to {} GiB",
                    v, DEFAULT_SIGNUP_STORAGE_GIB
                );
                DEFAULT_SIGNUP_STORAGE_GIB * GIB
            }
        },
    }
}

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
