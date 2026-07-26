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
    println!("[watchcat] online: orphan GC every {}s (TTL {}h)", interval, env_u64("WATCHCAT_GC_TTL_HOURS", 2));
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

async fn gc_orphan_uploads(pool: &Pool<Postgres>, r2: &R2) -> anyhow::Result<()> {
    // Floor at 1h so a misconfigured TTL of 0 can never reap uploads that are
    // still actively in progress.
    let ttl_hours = env_u64("WATCHCAT_GC_TTL_HOURS", 2).max(1) as i64;
    let batch = env_u64("WATCHCAT_GC_BATCH", 200).max(1) as i64;

    // ttl_hours/batch are our own integers: safe to inline into the interval.
    let q = format!(
        "SELECT id, user_id FROM files \
         WHERE deleted = false AND uploaded_chunks < total_chunks \
           AND created_on < NOW() - INTERVAL '{} hours' \
         ORDER BY created_on ASC LIMIT {}",
        ttl_hours, batch
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

        // Drop the file row; chunk rows cascade via the FK.
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

        match sqlx::query("DELETE FROM files WHERE id = $1").bind(file_id).execute(pool).await {
            Ok(_) => reaped_files += 1,
            Err(e) => eprintln!("[watchcat] delete trashed file {} failed: {:?}", file_id, e),
        }
    }

    // Folders only after their files, so an emptied folder disappears in the same
    // sweep and a folder still holding un-reaped files waits for the next one.
    let folder_q = format!(
        "DELETE FROM folders f \
         WHERE f.deleted = true AND f.deleted_on IS NOT NULL \
           AND f.deleted_on < NOW() - INTERVAL '{} days' \
           AND NOT EXISTS (SELECT 1 FROM files x WHERE x.folder_id = f.id) \
           AND NOT EXISTS (SELECT 1 FROM folders c WHERE c.parent_id = f.id)",
        ttl_days
    );
    let folder_res = sqlx::query(&folder_q).execute(pool).await?;

    if reaped_files > 0 || folder_res.rows_affected() > 0 {
        println!(
            "[watchcat] trash retention: reaped {} file(s), {} folder(s), removed {} object(s) (ttl {}d)",
            reaped_files,
            folder_res.rows_affected(),
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

    let res = sqlx::query(&format!(
        "DELETE FROM orders \
          WHERE LOWER(COALESCE(status, '')) NOT IN ('paid', 'completed', 'success') \
            AND invoice_number IS NULL \
            AND created_on < NOW() - INTERVAL '{} days'",
        ttl_days
    ))
    .execute(pool)
    .await?;

    if res.rows_affected() > 0 {
        println!(
            "[watchcat] removed {} abandoned order(s) older than {}d",
            res.rows_affected(),
            ttl_days
        );
    }
    Ok(())
}
