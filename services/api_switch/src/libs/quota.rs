//! Per-user storage quota checks.
//!
//! Authenticated uploads previously issued presigned PUT URLs with no quota
//! check, so any account could upload past its plan. `for_user` mirrors the
//! limit/used computation used by the storage-stats endpoint, and admission is
//! decided before an upload is accepted.
//!
//! `used` is summed from each chunk's real R2 object size (`size_on_server`),
//! which mark-chunk-complete records via HeadObject, so a client that
//! under-declares `file_size` cannot understate its usage. The create-time
//! pre-check admits on the declared `file_size` (the bytes aren't stored yet).
//! That is only sound because `libs::uploads` holds the declaration to what is
//! stored: chunk sizes must add up to it, each PUT URL is signed for its
//! chunk's exact length, and a chunk counts only once measured at that length.
//!
//! Nothing is stored when an upload is created, so `used` also counts every
//! chunk that is still waiting for its PUT, at the length its URL was signed
//! for, for as long as that URL works. Without that, any number of uploads
//! started before the first one landed were each checked against the same
//! stored total, and all of them fit.
//!
//! Trashed files do not count, though their bytes stay stored until the trash
//! is emptied, so restoring one checks the plan again (restore_files). A
//! trashed upload whose URLs still work keeps its room until they expire.
//!
//! Every path that admits bytes (create, a request-link create, restore, and a
//! chunk marked complete after its room lapsed) takes `lock` first and writes
//! in the same transaction, so requests sent together cannot each see the same
//! usage. An account over its plan, as a lapsed subscription leaves it, is
//! read-only: every one of those paths refuses it until it is under the plan.

use sqlx::{PgConnection, Postgres};

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

/// Serialise quota admission for one account until the caller's transaction
/// ends. Hold it from the usage check to the commit of whatever was admitted.
pub async fn lock(conn: &mut PgConnection, user_id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('silocat-quota:' || $1, 0))")
        .bind(user_id)
        .execute(conn)
        .await
        .map(|_| ())
}

/// How long a chunk that has not been marked uploaded keeps its room: the life
/// of its PUT URL plus the margin the orphan sweep also waits.
pub fn pending_window_secs() -> i64 {
    crate::libs::r2::PUT_URL_TTL.as_secs() as i64 + crate::libs::r2::PUT_URL_GRACE.as_secs() as i64
}

/// Compute a user's storage limit (base + active non-expired subscription space)
/// and current used bytes: stored chunks of non-deleted files, plus chunks whose
/// PUT URL still works, of any file. Returns `None` on a DB error or unknown
/// user. Run it after `lock` in the transaction that writes the admission.
pub async fn for_user(conn: &mut PgConnection, user_id: &str) -> Option<Quota> {
    let limit = sqlx::query_scalar::<Postgres, Option<i64>>(
        "SELECT ((SELECT default_storage_bytes FROM users WHERE id = $1) \
              + COALESCE((SELECT SUM(additional_space) FROM subscriptions \
                          WHERE created_by = $1 AND expires_on > NOW()), 0))::BIGINT",
    )
    .bind(user_id)
    .fetch_one(&mut *conn)
    .await
    .ok()
    .flatten()?;

    // Stored chunks count at their HeadObject-measured size_on_server, not the
    // client-declared size, and only while their file is not trashed. A chunk
    // still waiting for its PUT counts at the length its URL was signed for
    // (declared size, plus the tag when encrypted) until that URL expires,
    // trashed or not, since the URL still works either way.
    let used = sqlx::query_scalar::<Postgres, Option<i64>>(
        // GREATEST: a size recorded negative before sizes were checked must
        // not lower the total and with it every later admission.
        "SELECT COALESCE(SUM(CASE WHEN c.uploaded THEN GREATEST(c.size_on_server, 0) \
                                  ELSE GREATEST(c.size, 0) + CASE WHEN f.encrypted THEN $3 ELSE 0 END END), 0)::BIGINT \
         FROM chunks c JOIN files f ON f.id = c.file_id \
         WHERE f.user_id = $1 \
           AND ((c.uploaded = true AND f.deleted = false) \
             OR (c.uploaded = false AND c.created_on > NOW() - make_interval(secs => $2)))",
    )
    .bind(user_id)
    .bind(pending_window_secs() as f64)
    .bind(crate::libs::uploads::ENCRYPTION_TAG_BYTES)
    .fetch_one(&mut *conn)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    Some(Quota { limit, used })
}
