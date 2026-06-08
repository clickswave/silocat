//! Ban checks. Registered users are banned by id (columns on `users`);
//! anonymous users are banned by IP (the `ip_bans` table). Runtime queries only,
//! to stay SQLX_OFFLINE-safe.

use chrono::{DateTime, Utc};
use sqlx::{Pool, Postgres};

pub struct BanInfo {
    pub reason: Option<String>,
    pub until: Option<DateTime<Utc>>, // None = permanent
}

/// Active ban for a registered user, if any.
pub async fn user_ban(pool: &Pool<Postgres>, user_id: &str) -> Option<BanInfo> {
    let row = sqlx::query_as::<_, (bool, Option<DateTime<Utc>>, Option<String>)>(
        "SELECT is_banned, banned_until, ban_reason FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    let (is_banned, until, reason) = row;
    if !is_banned {
        return None;
    }
    // Temporary ban that has elapsed is no longer active.
    if let Some(u) = until {
        if u <= Utc::now() {
            return None;
        }
    }
    Some(BanInfo { reason, until })
}

/// Active IP ban, if any.
pub async fn ip_ban(pool: &Pool<Postgres>, ip: &str) -> Option<BanInfo> {
    let row = sqlx::query_as::<_, (Option<String>, Option<DateTime<Utc>>)>(
        "SELECT reason, banned_until FROM ip_bans \
         WHERE ip = $1 AND (banned_until IS NULL OR banned_until > NOW()) \
         ORDER BY created_at DESC LIMIT 1",
    )
    .bind(ip)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;
    Some(BanInfo { reason: row.0, until: row.1 })
}

/// Whether a registered user is restricted (read-only: no uploads).
pub async fn user_restricted(pool: &Pool<Postgres>, user_id: &str) -> bool {
    sqlx::query_scalar::<_, bool>("SELECT COALESCE(is_restricted, false) FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(false)
}
