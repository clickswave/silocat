//! WatchCat: SiloCat's background scheduler. Runs as its own container (the
//! api_switch binary started with WATCHCAT_MODE=1), so scheduled housekeeping is
//! isolated from the request path. More jobs will live here over time.
//!
//! Job 1: orphan-upload GC: an upload creates a `files` row + `chunks` rows up
//! front, then PUTs each chunk to R2. If the user abandons it, completed chunks
//! sit in R2 forever and the half-finished file lingers in the DB. This reaps
//! any incomplete upload older than the TTL: delete every expected chunk object
//! from its bucket (covers "ghost" objects whose mark-complete never landed),
//! then delete the file row (chunks cascade via FK).

use crate::libs::r2::R2;
use sqlx::{Pool, Postgres};
use std::time::Duration;
use tokio::time::sleep;

fn env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(default)
}

/// Scheduler entrypoint. Never returns.
pub async fn run(pool: Pool<Postgres>, r2: R2) {
    let interval = env_u64("WATCHCAT_GC_INTERVAL_SECS", 900); // every 15 min
    println!("[watchcat] online: orphan GC every {}s (TTL {} min)", interval, orphan_ttl_minutes());
    // Warm-up so we don't race api_switch's migrations on a cold start.
    sleep(Duration::from_secs(8)).await;
    loop {
        if let Err(e) = gc_orphan_uploads(&pool, &r2).await {
            eprintln!("[watchcat] orphan gc failed: {:?}", e);
        }
        if let Err(e) = gc_expired_shadow_files(&pool, &r2).await {
            eprintln!("[watchcat] shadow ttl gc failed: {:?}", e);
        }
        if let Err(e) = downgrade_expired_subscriptions(&pool).await {
            eprintln!("[watchcat] subscription downgrade failed: {:?}", e);
        }
        if let Err(e) = gc_expired_trash(&pool, &r2).await {
            eprintln!("[watchcat] trash retention gc failed: {:?}", e);
        }
        if let Err(e) = gc_abandoned_orders(&pool).await {
            eprintln!("[watchcat] abandoned order gc failed: {:?}", e);
        }
        sleep(Duration::from_secs(interval)).await;
    }
}

/// How old an unfinished upload must be before the orphan sweep reaps it.
///
/// Never less than the life of its PUT URLs plus the grace margin: deleting the
/// rows first would let a late PUT land an object that no row, quota or sweep
/// ever finds again. Never more than 30 days, so a huge WATCHCAT_GC_TTL_HOURS
/// still makes an interval Postgres accepts.
fn orphan_ttl_minutes() -> u64 {
    const MAX_MINUTES: u64 = 30 * 24 * 60;
    let floor = (crate::libs::r2::PUT_URL_TTL + crate::libs::r2::PUT_URL_GRACE).as_secs() / 60;
    env_u64("WATCHCAT_GC_TTL_HOURS", 2).saturating_mul(60).clamp(floor, MAX_MINUTES)
}

async fn gc_orphan_uploads(pool: &Pool<Postgres>, r2: &R2) -> anyhow::Result<()> {
    let ttl_minutes = orphan_ttl_minutes();
    let batch = env_u64("WATCHCAT_GC_BATCH", 200).max(1) as i64;

    // ttl_minutes/batch are our own integers: safe to inline into the interval.
    // Trashed uploads too: an unfinished file is no use in the trash, and an
    // upload through a request link holds a slot on that link until reaped.
    let q = format!(
        "SELECT id, user_id FROM files \
         WHERE uploaded_chunks < total_chunks \
           AND created_on < NOW() - INTERVAL '{} minutes' \
         ORDER BY created_on ASC LIMIT {}",
        ttl_minutes, batch
    );
    let files: Vec<(String, Option<String>)> = sqlx::query_as(&q).fetch_all(pool).await?;
    if files.is_empty() {
        return Ok(());
    }

    let mut reaped = 0usize;
    let mut objects = 0usize;
    for (file_id, user_id) in &files {
        // Logged-in uploads live in sanctum; anonymous in shadow (mirrors create_files).
        let storage = if user_id.is_some() { "sanctum" } else { "shadow" };

        // Delete every chunk object for this file. The R2 key is the chunk id.
        // We delete all chunk keys (not just uploaded=true) so a ghost object
        // whose mark-complete failed is still removed. delete_object is idempotent.
        let chunk_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM chunks WHERE file_id = $1")
            .bind(file_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        for cid in &chunk_ids {
            match r2.delete_object(storage, cid).await {
                Ok(_) => objects += 1,
                Err(e) => eprintln!("[watchcat] r2 delete {}/{} failed: {:?}", storage, cid, e),
            }
        }

        // An upload through a request link took one of the request's slots
        // when it started, so the slot goes back first. Then drop the file
        // row; chunk rows cascade via the FK.
        if let Err(e) = crate::libs::uploads::release_request_slots(pool, std::slice::from_ref(file_id)).await {
            eprintln!("[watchcat] release request slot of {} failed: {:?}", file_id, e);
            continue;
        }
        match sqlx::query("DELETE FROM files WHERE id = $1").bind(file_id).execute(pool).await {
            Ok(_) => reaped += 1,
            Err(e) => eprintln!("[watchcat] delete file {} failed: {:?}", file_id, e),
        }
    }

    println!("[watchcat] orphan gc: reaped {} upload(s), removed {} chunk object(s)", reaped, objects);
    Ok(())
}

/// Job 2: shadow TTL GC: anonymous ("shadow") uploads are not permanent
/// storage. This reaps completed shadow files (`user_id IS NULL`) older than the
/// TTL, deleting their chunk objects from the shadow bucket and the file rows
/// (chunks cascade). Bounds unbounded free-storage growth / cost.
async fn gc_expired_shadow_files(pool: &Pool<Postgres>, r2: &R2) -> anyhow::Result<()> {
    // 7 days, matching what the landing page, pricing footnote and upload
    // success modal all promise senders. Changing this default without changing
    // that copy makes the product lie about how long it keeps your files.
    let ttl_days = env_u64("WATCHCAT_SHADOW_TTL_DAYS", 7).max(1) as i64;
    let batch = env_u64("WATCHCAT_GC_BATCH", 200).max(1) as i64;

    // ttl_days/batch are our own integers: safe to inline into the interval.
    let q = format!(
        "SELECT id FROM files \
         WHERE user_id IS NULL AND deleted = false \
           AND created_on < NOW() - INTERVAL '{} days' \
         ORDER BY created_on ASC LIMIT {}",
        ttl_days, batch
    );
    let files: Vec<(String,)> = sqlx::query_as(&q).fetch_all(pool).await?;
    if files.is_empty() {
        return Ok(());
    }

    let mut reaped = 0usize;
    let mut objects = 0usize;
    for (file_id,) in &files {
        let chunk_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM chunks WHERE file_id = $1")
            .bind(file_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();
        for cid in &chunk_ids {
            match r2.delete_object("shadow", cid).await {
                Ok(_) => objects += 1,
                Err(e) => eprintln!("[watchcat] r2 delete shadow/{} failed: {:?}", cid, e),
            }
        }
        match sqlx::query("DELETE FROM files WHERE id = $1").bind(file_id).execute(pool).await {
            Ok(_) => reaped += 1,
            Err(e) => eprintln!("[watchcat] delete shadow file {} failed: {:?}", file_id, e),
        }
    }

    println!("[watchcat] shadow ttl gc: reaped {} file(s), removed {} object(s)", reaped, objects);
    Ok(())
}

/// Job 4: trash retention: items sit in the trash for a fixed window and then
/// delete themselves, which is what the Trash screen promises ("Items stay here
/// for 30 days, then delete themselves") and what the per-row countdown counts
/// down to. Without this the promise is a lie and trash grows forever.
///
/// Files carry their ciphertext in R2, so the chunk objects go first and the row
/// second; a failed object delete leaves the row in place to be retried next
/// sweep rather than orphaning the blobs.
async fn gc_expired_trash(pool: &Pool<Postgres>, r2: &R2) -> anyhow::Result<()> {
    // Floor at 1 day: a misconfigured 0 must never make the trash a no-op bin
    // that discards things the moment they are deleted.
    let ttl_days = env_u64("WATCHCAT_TRASH_TTL_DAYS", 30).max(1) as i64;
    let batch = env_u64("WATCHCAT_GC_BATCH", 200).max(1) as i64;

    // ttl_days/batch are our own integers: safe to inline into the interval.
    let file_q = format!(
        "SELECT id, user_id FROM files \
         WHERE deleted = true AND deleted_on IS NOT NULL \
           AND deleted_on < NOW() - INTERVAL '{} days' \
         ORDER BY deleted_on ASC LIMIT {}",
        ttl_days, batch
    );
    let files: Vec<(String, Option<String>)> = sqlx::query_as(&file_q).fetch_all(pool).await?;

    let mut reaped_files = 0usize;
    let mut objects = 0usize;
    for (file_id, user_id) in &files {
        // Account files live in `sanctum`, anonymous drops in `shadow`.
        let storage = if user_id.is_some() { "sanctum" } else { "shadow" };

        let chunk_ids: Vec<String> = sqlx::query_scalar("SELECT id FROM chunks WHERE file_id = $1")
            .bind(file_id)
            .fetch_all(pool)
            .await
            .unwrap_or_default();

        let mut all_gone = true;
        for cid in &chunk_ids {
            match r2.delete_object(storage, cid).await {
                Ok(_) => objects += 1,
                Err(e) => {
                    all_gone = false;
                    eprintln!("[watchcat] r2 delete {}/{} failed: {:?}", storage, cid, e);
                }
            }
        }
        // Keep the row if any object survived, so the next sweep retries instead
        // of leaving unreferenced ciphertext paid for and unreachable.
        if !all_gone {
            continue;
        }

        if let Err(e) = crate::libs::uploads::release_request_slots(pool, std::slice::from_ref(file_id)).await {
            eprintln!("[watchcat] release request slot of {} failed: {:?}", file_id, e);
            continue;
        }
        match sqlx::query("DELETE FROM files WHERE id = $1").bind(file_id).execute(pool).await {
            Ok(_) => reaped_files += 1,
            Err(e) => eprintln!("[watchcat] delete trashed file {} failed: {:?}", file_id, e),
        }
    }

    // A trashed folder goes with everything under it, as a permanent delete
    // from the Trash screen does. Trashing marks only the folder's own row, so
    // its contents are not in the trash themselves; waiting for the folder to
    // empty meant it never did, and links inside it outlived the 30 days.
    let folder_q = format!(
        "SELECT id FROM folders \
         WHERE deleted = true AND deleted_on IS NOT NULL \
           AND deleted_on < NOW() - INTERVAL '{} days' \
         ORDER BY deleted_on ASC LIMIT {}",
        ttl_days, batch
    );
    let folders: Vec<(String,)> = sqlx::query_as(&folder_q).fetch_all(pool).await?;

    let mut reaped_folders = 0u64;
    for (folder_id,) in &folders {
        // Every chunk in the subtree first, since the rows cascade away with the
        // folder and the objects would be left behind. UNION, not UNION ALL, so
        // a parent_id cycle ends the walk instead of looping.
        let chunks: Vec<(String, Option<String>)> = sqlx::query_as(
            "WITH RECURSIVE subtree AS ( \
                 SELECT id FROM folders WHERE id = $1 \
                 UNION \
                 SELECT f.id FROM folders f JOIN subtree s ON f.parent_id = s.id \
             ) \
             SELECT c.id, fi.user_id FROM files fi \
             JOIN chunks c ON c.file_id = fi.id \
             WHERE fi.folder_id IN (SELECT id FROM subtree)",
        )
        .bind(folder_id)
        .fetch_all(pool)
        .await?;

        let mut all_gone = true;
        for (cid, user_id) in &chunks {
            let storage = if user_id.is_some() { "sanctum" } else { "shadow" };
            match r2.delete_object(storage, cid).await {
                Ok(_) => objects += 1,
                Err(e) => {
                    all_gone = false;
                    eprintln!("[watchcat] r2 delete {}/{} failed: {:?}", storage, cid, e);
                }
            }
        }
        if !all_gone {
            continue;
        }

        if let Err(e) = crate::libs::uploads::release_request_slots_in_folder(pool, folder_id).await {
            eprintln!("[watchcat] release request slots in folder {} failed: {:?}", folder_id, e);
            continue;
        }
        // Folders, files and chunks below cascade.
        match sqlx::query("DELETE FROM folders WHERE id = $1").bind(folder_id).execute(pool).await {
            Ok(res) => reaped_folders += res.rows_affected(),
            Err(e) => eprintln!("[watchcat] delete trashed folder {} failed: {:?}", folder_id, e),
        }
    }

    if reaped_files > 0 || reaped_folders > 0 {
        println!(
            "[watchcat] trash retention: reaped {} file(s), {} folder(s), removed {} object(s) (ttl {}d)",
            reaped_files,
            reaped_folders,
            objects,
            ttl_days
        );
    }
    Ok(())
}

/// Job 3: subscription downgrade: detach expired subscriptions from users so an
/// account cleanly reverts to Free. The request-path token load already ignores
/// expired subscriptions; this keeps the stored pointer consistent.
async fn downgrade_expired_subscriptions(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    let res = sqlx::query(
        "UPDATE users SET subscription_id = NULL \
         WHERE subscription_id IS NOT NULL \
           AND subscription_id IN (SELECT id FROM subscriptions WHERE expires_on <= NOW())",
    )
    .execute(pool)
    .await?;
    if res.rows_affected() > 0 {
        println!("[watchcat] downgraded {} expired subscription(s)", res.rows_affected());
    }
    Ok(())
}

/// Job 5: abandoned checkouts.
///
/// An order row is created when someone opens the payment sheet, so most of
/// them are people who changed their mind. They are not receipts and nothing
/// references them once they lapse: the user simply starts a new checkout,
/// which mints a fresh order. Left alone they accumulate forever and make the
/// orders table mostly noise.
///
/// Only unsettled orders are touched. Anything that reached paid, completed or
/// success is a financial record and is kept regardless of age, as is anything
/// holding an invoice number.
async fn gc_abandoned_orders(pool: &Pool<Postgres>) -> anyhow::Result<()> {
    // Floor at 1 day so a misconfigured TTL cannot delete a checkout that is
    // still in flight, waiting on a slow gateway webhook.
    let ttl_days = env_u64("WATCHCAT_ORDER_TTL_DAYS", 7).max(1) as i64;

    // Each order took one use of its promo code when it was written, so the
    // uses of the orders removed here go back in the same statement.
    let removed: i64 = sqlx::query_scalar(&format!(
        "WITH gone AS ( \
           DELETE FROM orders \
            WHERE NOT order_is_settled(status) \
              AND status <> 'refunded' \
              AND invoice_number IS NULL \
              AND created_on < NOW() - INTERVAL '{} days' \
           RETURNING details->>'promo_code' AS code), \
         released AS ( \
           UPDATE promo_codes p SET uses_count = GREATEST(p.uses_count - g.n, 0) \
             FROM (SELECT code, COUNT(*)::INT AS n FROM gone WHERE code IS NOT NULL GROUP BY code) g \
            WHERE p.code = g.code) \
         SELECT COUNT(*) FROM gone",
        ttl_days
    ))
    .fetch_one(pool)
    .await?;

    if removed > 0 {
        println!(
            "[watchcat] removed {} abandoned order(s) older than {}d",
            removed,
            ttl_days
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_orphan_ttl_stays_between_the_url_life_and_thirty_days() {
        // Only this test reads WATCHCAT_GC_TTL_HOURS.
        unsafe { std::env::set_var("WATCHCAT_GC_TTL_HOURS", u64::MAX.to_string()) };
        let huge = orphan_ttl_minutes();
        unsafe { std::env::set_var("WATCHCAT_GC_TTL_HOURS", "1") };
        let small = orphan_ttl_minutes();
        unsafe { std::env::remove_var("WATCHCAT_GC_TTL_HOURS") };
        assert_eq!(huge, 30 * 24 * 60);
        assert_eq!(small, 135);
    }

    /// Trashing a folder marks that one row, so whatever is inside stays
    /// live. The sweep only took folders with nothing left in them and only
    /// files marked deleted themselves, so a trashed folder with anything in
    /// it was never purged. Runs against a Postgres named by
    /// SILOCAT_TEST_DATABASE_URL and skips when it is unset.
    #[tokio::test]
    async fn an_expired_trashed_folder_is_purged_with_everything_in_it() {
        let Some(st) = crate::test_support::state().await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        let pool = &st.pg_pool;
        let (user, _key) = crate::test_support::account(pool).await;

        async fn folder(pool: &Pool<Postgres>, user: &str, parent: Option<&str>, trashed_days_ago: Option<i32>) -> String {
            sqlx::query_scalar(
                "INSERT INTO folders (name, user_id, parent_id, deleted, deleted_on) \
                 VALUES ('dir', $1, $2, $3 IS NOT NULL, NOW() - make_interval(days => $3)) RETURNING id",
            )
            .bind(user)
            .bind(parent)
            .bind(trashed_days_ago)
            .fetch_one(pool)
            .await
            .expect("seed folder")
        }
        async fn file(pool: &Pool<Postgres>, user: &str, folder_id: &str) -> String {
            sqlx::query_scalar(
                "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, \
                 sha256_checksum, blake3_checksum, public_access, folder_id) \
                 VALUES ($1, 'f.bin', 'application/octet-stream', 1, false, 1, 1, 's', '', false, $2) RETURNING id",
            )
            .bind(user)
            .bind(folder_id)
            .fetch_one(pool)
            .await
            .expect("seed file")
        }

        let expired = folder(pool, &user, None, Some(31)).await;
        let child = folder(pool, &user, Some(&expired), None).await;
        let deep_file = file(pool, &user, &child).await;
        let top_file = file(pool, &user, &expired).await;
        let recent = folder(pool, &user, None, Some(2)).await;
        let recent_file = file(pool, &user, &recent).await;

        gc_expired_trash(pool, &st.r2).await.expect("sweep");

        let mut left = vec![];
        for (what, sql, id) in [
            ("expired folder", "SELECT count(*) FROM folders WHERE id = $1", &expired),
            ("its child folder", "SELECT count(*) FROM folders WHERE id = $1", &child),
            ("file in the child", "SELECT count(*) FROM files WHERE id = $1", &deep_file),
            ("file at the top", "SELECT count(*) FROM files WHERE id = $1", &top_file),
            ("recent folder", "SELECT count(*) FROM folders WHERE id = $1", &recent),
            ("file in the recent folder", "SELECT count(*) FROM files WHERE id = $1", &recent_file),
        ] {
            let n: i64 = sqlx::query_scalar(sql).bind(id).fetch_one(pool).await.unwrap();
            left.push((what, n));
        }
        let _ = sqlx::query("DELETE FROM files WHERE user_id = $1").bind(&user).execute(pool).await;
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(&user).execute(pool).await;
        crate::test_support::forget(pool, &user).await;

        assert_eq!(
            left,
            vec![
                ("expired folder", 0),
                ("its child folder", 0),
                ("file in the child", 0),
                ("file at the top", 0),
                ("recent folder", 1),
                ("file in the recent folder", 1),
            ]
        );
    }

    /// An upload through a capped request link holds a slot from the moment
    /// it starts. When the sweep reaps it unfinished, the slot goes back, or a
    /// stalled upload would close the link for the people it was sent to.
    #[tokio::test]
    async fn reaping_an_unfinished_request_upload_gives_its_slot_back() {
        let Some(st) = crate::test_support::state().await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        let pool = &st.pg_pool;
        let (owner, _key) = crate::test_support::account(pool).await;
        let token = crate::libs::rng::uuid().replace('-', "");
        let request: String = sqlx::query_scalar(
            "INSERT INTO file_requests (token, owner_user_id, max_uploads, upload_count) \
             VALUES ($1, $2, 1, 1) RETURNING id",
        )
        .bind(&token)
        .bind(&owner)
        .fetch_one(pool)
        .await
        .expect("seed request");
        let stalled: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, \
             sha256_checksum, blake3_checksum, public_access, created_on) \
             VALUES ($1, 'f.bin', 'application/octet-stream', 1, false, 1, 0, 's', '', false, \
             NOW() - INTERVAL '31 days') RETURNING id",
        )
        .bind(&owner)
        .fetch_one(pool)
        .await
        .expect("seed file");
        sqlx::query("INSERT INTO request_uploads (request_id, file_id) VALUES ($1, $2)")
            .bind(&request)
            .bind(&stalled)
            .execute(pool)
            .await
            .expect("seed request upload");

        gc_orphan_uploads(pool, &st.r2).await.expect("sweep");

        let files: i64 = sqlx::query_scalar("SELECT count(*) FROM files WHERE id = $1")
            .bind(&stalled)
            .fetch_one(pool)
            .await
            .unwrap();
        let count: i32 = sqlx::query_scalar("SELECT upload_count FROM file_requests WHERE id = $1")
            .bind(&request)
            .fetch_one(pool)
            .await
            .unwrap();
        crate::test_support::forget(pool, &owner).await;
        assert_eq!(files, 0);
        assert_eq!(count, 0);
    }

    /// A capped link with one slot taken by an unfinished upload of the given
    /// age, `trashed_days_ago` days in the trash if set. Returns (owner,
    /// request id, file id).
    async fn slot_held_by_unfinished_upload(
        pool: &Pool<Postgres>,
        age_minutes: i32,
        trashed_days_ago: Option<i32>,
    ) -> (String, String, String) {
        let (owner, _key) = crate::test_support::account(pool).await;
        let token = crate::libs::rng::uuid().replace('-', "");
        let request: String = sqlx::query_scalar(
            "INSERT INTO file_requests (token, owner_user_id, max_uploads, upload_count) \
             VALUES ($1, $2, 1, 1) RETURNING id",
        )
        .bind(&token)
        .bind(&owner)
        .fetch_one(pool)
        .await
        .expect("seed request");
        let file: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, name, mime, size, encrypted, total_chunks, uploaded_chunks, \
             sha256_checksum, blake3_checksum, public_access, created_on, deleted, deleted_on) \
             VALUES ($1, 'f.bin', 'application/octet-stream', 1, false, 1, 0, 's', '', false, \
             NOW() - make_interval(mins => $2), $3 IS NOT NULL, NOW() - make_interval(days => $3)) RETURNING id",
        )
        .bind(&owner)
        .bind(age_minutes)
        .bind(trashed_days_ago)
        .fetch_one(pool)
        .await
        .expect("seed file");
        sqlx::query("INSERT INTO request_uploads (request_id, file_id) VALUES ($1, $2)")
            .bind(&request)
            .bind(&file)
            .execute(pool)
            .await
            .expect("seed request upload");
        (owner, request, file)
    }

    async fn slots_used(pool: &Pool<Postgres>, request: &str) -> i32 {
        sqlx::query_scalar("SELECT upload_count FROM file_requests WHERE id = $1")
            .bind(request)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    async fn file_exists(pool: &Pool<Postgres>, file: &str) -> bool {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM files WHERE id = $1)")
            .bind(file)
            .fetch_one(pool)
            .await
            .unwrap()
    }

    /// The owner can trash a file that is still arriving through a request
    /// link. The sweep skipped trashed files, so its slot was never given back
    /// and a link capped at one stayed full for good.
    #[tokio::test]
    async fn reaping_a_trashed_unfinished_request_upload_gives_its_slot_back() {
        let Some(st) = crate::test_support::state().await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        let pool = &st.pg_pool;
        let (owner, request, file) = slot_held_by_unfinished_upload(pool, 31 * 24 * 60, Some(0)).await;

        gc_orphan_uploads(pool, &st.r2).await.expect("sweep");

        let gone = !file_exists(pool, &file).await;
        let count = slots_used(pool, &request).await;
        crate::test_support::forget(pool, &owner).await;
        assert!(gone);
        assert_eq!(count, 0);
    }

    /// Trash retention deletes rows outright, so an unfinished request upload
    /// it takes must give its slot back as the orphan sweep's would.
    #[tokio::test]
    async fn trash_retention_gives_back_the_slot_of_an_unfinished_request_upload() {
        let Some(mut st) = crate::test_support::state().await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        // Storage that accepts deletes, or retention keeps the row to retry.
        st.r2 = crate::test_support::storage_holding(0);
        let pool = &st.pg_pool;
        // Young enough that the orphan sweep of a test running alongside
        // leaves it alone.
        let (owner, request, file) = slot_held_by_unfinished_upload(pool, 0, Some(31)).await;

        gc_expired_trash(pool, &st.r2).await.expect("retention");

        let gone = !file_exists(pool, &file).await;
        let count = slots_used(pool, &request).await;
        crate::test_support::forget(pool, &owner).await;
        assert!(gone);
        assert_eq!(count, 0);
    }

    /// An order reserves one use of its promo code when it is written. Sweeping
    /// an abandoned order must hand that use back, or anyone could spend a
    /// capped code with checkouts they never pay. Runs against a Postgres named
    /// by SILOCAT_TEST_DATABASE_URL and skips when it is unset.
    #[tokio::test]
    async fn sweeping_abandoned_orders_gives_their_promo_uses_back() {
        let Some(st) = crate::test_support::state().await else {
            return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL");
        };
        let pool = &st.pg_pool;
        let (user, _key) = crate::test_support::account(pool).await;
        let code = format!("GC{}", crate::libs::rng::string(10)).to_uppercase();
        sqlx::query("INSERT INTO promo_codes (code, discount_percentage, duration, active, max_uses, uses_count) VALUES ($1, 50, '1 month', TRUE, 4, 4)")
            .bind(&code)
            .execute(pool)
            .await
            .unwrap();
        // Two abandoned checkouts and one failed payment, all past the TTL, and
        // one checkout still in flight.
        for (status, age) in [("pending", 30), ("pending", 30), ("failed", 30), ("pending", 0)] {
            sqlx::query(&format!(
                "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, payment_gateway, \
                 currency, amount, status, details, created_on) \
                 VALUES ($1, $2, 'pro', 'monthly', 'razorpay', 'USD', 500, $3, $4, NOW() - INTERVAL '{} days')",
                age
            ))
            .bind(format!("order_{}", crate::libs::rng::string(14)))
            .bind(&user)
            .bind(status)
            .bind(serde_json::json!({ "promo_code": code }))
            .execute(pool)
            .await
            .unwrap();
        }
        gc_abandoned_orders(pool).await.unwrap();
        let used: i32 = sqlx::query_scalar("SELECT uses_count FROM promo_codes WHERE code = $1")
            .bind(&code)
            .fetch_one(pool)
            .await
            .unwrap();
        let left: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE user_id = $1")
            .bind(&user)
            .fetch_one(pool)
            .await
            .unwrap();
        sqlx::query("DELETE FROM orders WHERE user_id = $1").bind(&user).execute(pool).await.unwrap();
        sqlx::query("DELETE FROM promo_codes WHERE code = $1").bind(&code).execute(pool).await.unwrap();
        crate::test_support::forget(pool, &user).await;
        assert_eq!(left, 1, "only the checkout still in flight stays");
        assert_eq!(used, 1, "the three swept orders give their uses back");
    }
}
