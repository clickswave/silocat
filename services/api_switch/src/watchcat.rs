//! WatchCat: SiloCat's background scheduler. Runs as its own container (the
//! api_switch binary started with WATCHCAT_MODE=1), so scheduled housekeeping is
//! isolated from the request path. More jobs will live here over time.
//!
//! Job 1 — orphan-upload GC: an upload creates a `files` row + `chunks` rows up
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
    println!("[watchcat] online — orphan GC every {}s (TTL {}h)", interval, env_u64("WATCHCAT_GC_TTL_HOURS", 2));
    // Warm-up so we don't race api_switch's migrations on a cold start.
    sleep(Duration::from_secs(8)).await;
    loop {
        if let Err(e) = gc_orphan_uploads(&pool, &r2).await {
            eprintln!("[watchcat] orphan gc failed: {:?}", e);
        }
        sleep(Duration::from_secs(interval)).await;
    }
}

async fn gc_orphan_uploads(pool: &Pool<Postgres>, r2: &R2) -> anyhow::Result<()> {
    let ttl_hours = env_u64("WATCHCAT_GC_TTL_HOURS", 2).max(0) as i64;
    let batch = env_u64("WATCHCAT_GC_BATCH", 200).max(1) as i64;

    // ttl_hours/batch are our own integers — safe to inline into the interval.
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
