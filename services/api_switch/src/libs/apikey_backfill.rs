//! One-shot conversion of plaintext API keys to blind index + ciphertext.
//!
//! Runs at startup, before the server accepts traffic. Rows are converted one
//! at a time inside a transaction, because a key's plaintext also appears in
//! `files.owner_api_key` / `folders.owner_api_key` (that column *is* the
//! ownership record for anonymous uploads). Rewriting the user row without
//! rewriting the object rows in the same transaction would orphan every
//! anonymous upload that key owns.
//!
//! Idempotent: converted rows are marked and skipped, so restarts are safe and
//! the whole thing becomes a no-op once the fleet has rolled forward.

use crate::libs::apikey;

/// Convert any remaining plaintext keys. Returns (users, anonymous) converted.
pub async fn run(pool: &sqlx::PgPool) -> Result<(usize, usize), sqlx::Error> {
    let users = migrate_users(pool).await?;
    let anon = migrate_anonymous(pool).await?;
    let orphans = migrate_orphaned_owners(pool).await?;
    if users > 0 || anon > 0 {
        println!("[apikey] migrated {} user and {} anonymous keys to blind index", users, anon);
    }
    if orphans > 0 {
        println!("[apikey] indexed {} orphaned owner_api_key values", orphans);
    }
    Ok((users, anon))
}

/// Convert `owner_api_key` values whose owning row is already gone.
///
/// Anonymous uploads outlive their `anonymous_users` row once retention sweeps
/// it, leaving objects that reference a key nothing can authenticate as. The
/// two passes above only see keys that still have an owner row, so these would
/// keep their plaintext forever. They are unreachable either way, but a
/// plaintext bearer credential in the database is the thing this whole change
/// exists to remove, so index them too.
///
/// A stored value is already converted iff it is a 64-char hex digest; raw keys
/// are UUIDs, so the two cannot be confused.
async fn migrate_orphaned_owners(pool: &sqlx::PgPool) -> Result<usize, sqlx::Error> {
    let mut done = 0usize;
    for table in ["files", "folders"] {
        loop {
            let sql = format!(
                "SELECT DISTINCT owner_api_key FROM {table} \
                 WHERE owner_api_key IS NOT NULL AND owner_api_key !~ '^[0-9a-f]{{64}}$' LIMIT 500"
            );
            let batch: Vec<(String,)> = sqlx::query_as(&sql).fetch_all(pool).await?;
            if batch.is_empty() {
                break;
            }
            for (raw,) in &batch {
                let Some(bi) = apikey::blind_index(raw) else { return Ok(done) };
                let upd = format!("UPDATE {table} SET owner_api_key = $1 WHERE owner_api_key = $2");
                sqlx::query(&upd).bind(&bi).bind(raw).execute(pool).await?;
                done += 1;
            }
        }
    }
    Ok(done)
}

async fn migrate_users(pool: &sqlx::PgPool) -> Result<usize, sqlx::Error> {
    let mut done = 0usize;
    loop {
        let batch: Vec<(String, String)> = sqlx::query_as(
            "SELECT id, api_key FROM users \
             WHERE api_key_migrated = FALSE AND api_key IS NOT NULL LIMIT 500",
        )
        .fetch_all(pool)
        .await?;
        if batch.is_empty() {
            break;
        }

        for (id, raw) in &batch {
            let (Some(bi), Some(enc)) = (apikey::blind_index(raw), apikey::encrypt(raw)) else {
                // Misconfigured key material: stop rather than mangle rows.
                return Ok(done);
            };
            let mut tx = pool.begin().await?;
            // Objects first: they still reference the plaintext.
            sqlx::query("UPDATE files SET owner_api_key = $1 WHERE owner_api_key = $2")
                .bind(&bi).bind(raw).execute(&mut *tx).await?;
            sqlx::query("UPDATE folders SET owner_api_key = $1 WHERE owner_api_key = $2")
                .bind(&bi).bind(raw).execute(&mut *tx).await?;
            sqlx::query(
                "UPDATE users SET api_key = $1, api_key_enc = $2, api_key_migrated = TRUE WHERE id = $3",
            )
            .bind(&bi).bind(&enc).bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
            done += 1;
        }
    }
    Ok(done)
}

async fn migrate_anonymous(pool: &sqlx::PgPool) -> Result<usize, sqlx::Error> {
    let mut done = 0usize;
    loop {
        let batch: Vec<(String,)> = sqlx::query_as(
            "SELECT api_key FROM anonymous_users WHERE api_key_migrated = FALSE LIMIT 500",
        )
        .fetch_all(pool)
        .await?;
        if batch.is_empty() {
            break;
        }

        for (raw,) in &batch {
            let Some(bi) = apikey::blind_index(raw) else { return Ok(done) };
            let mut tx = pool.begin().await?;
            sqlx::query("UPDATE files SET owner_api_key = $1 WHERE owner_api_key = $2")
                .bind(&bi).bind(raw).execute(&mut *tx).await?;
            sqlx::query("UPDATE folders SET owner_api_key = $1 WHERE owner_api_key = $2")
                .bind(&bi).bind(raw).execute(&mut *tx).await?;
            // api_key IS the primary key here. On a live DB the server can mint a
            // fresh raw row for a returning anonymous user whose twin was already
            // migrated to `bi` on an earlier pass, so `UPDATE ... SET api_key = bi`
            // would collide on the PK and abort the whole migration. blind_index is
            // injective, so an existing `bi` row means the SAME key: this raw row is
            // a duplicate of it. Its objects are now re-pointed to `bi`, so drop the
            // duplicate rather than collide. Otherwise migrate it in place (anonymous
            // keys are browser-generated and never displayed, so index only).
            let bi_exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM anonymous_users WHERE api_key = $1)",
            )
            .bind(&bi)
            .fetch_one(&mut *tx)
            .await?;
            if bi_exists {
                sqlx::query("DELETE FROM anonymous_users WHERE api_key = $1")
                    .bind(raw)
                    .execute(&mut *tx)
                    .await?;
            } else {
                sqlx::query(
                    "UPDATE anonymous_users SET api_key = $1, api_key_migrated = TRUE WHERE api_key = $2",
                )
                .bind(&bi).bind(raw).execute(&mut *tx).await?;
            }
            tx.commit().await?;
            done += 1;
        }
    }
    Ok(done)
}
