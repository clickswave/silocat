//! Admin: silo.cat accounts as the console shows them.
//!
//!   GET  /admin/accounts                  -> a page of accounts with usage, links, plan
//!   GET  /admin/accounts/{id}             -> one account, its invoices and recent bandwidth
//!   POST /admin/accounts/{id}/quota       -> raise (or lower) the base storage quota
//!   POST /admin/accounts/{id}/disable-links -> turn off every share link the account has
//!
//! `/admin/users` stays for the old panel. This listing exists because that one
//! returns raw rows capped at 100 with no usage at all, so the console could not
//! say who is near their quota, who shares publicly, or who has gone quiet.

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list))
        .route("/{id}", get(detail))
        .route("/{id}/quota", post(quota))
        .route("/{id}/disable-links", post(disable_links))
}

/// One row per account with everything the console's cards and table show.
/// Used bytes are measured the way libs::quota measures them (stored chunks of
/// files not deleted), and the quota is the same base plus active subscription
/// space, so the console and the upload limit can never disagree.
const ACCOUNTS: &str = "\
WITH used AS ( \
    SELECT f.user_id, SUM(c.size_on_server)::BIGINT AS used \
    FROM chunks c JOIN files f ON f.id = c.file_id \
    WHERE f.deleted = false AND c.uploaded = true AND f.user_id IS NOT NULL \
    GROUP BY f.user_id), \
fs AS ( \
    SELECT user_id, COUNT(*)::BIGINT AS files, \
        COUNT(*) FILTER (WHERE share_type IS DISTINCT FROM 'off' AND share_token IS NOT NULL \
            AND (share_expires_at IS NULL OR share_expires_at > NOW()))::BIGINT AS public_links, \
        COALESCE(SUM(size) FILTER (WHERE encrypted), 0)::BIGINT AS enc_bytes, \
        COALESCE(SUM(size), 0)::BIGINT AS bytes \
    FROM files WHERE deleted = false AND user_id IS NOT NULL GROUP BY user_id), \
sub AS ( \
    SELECT created_by, SUM(additional_space)::BIGINT AS extra, MAX(expires_on) AS renews, MAX(name) AS plan \
    FROM subscriptions WHERE expires_on > NOW() GROUP BY created_by), \
req AS ( \
    SELECT owner_user_id, COUNT(*)::BIGINT AS request_links FROM file_requests \
    WHERE active AND (expires_at IS NULL OR expires_at > NOW()) GROUP BY owner_user_id), \
base AS ( \
    SELECT u.id, u.username, u.email, u.created_on, u.email_verified, u.is_restricted, \
        (u.is_banned AND (u.banned_until IS NULL OR u.banned_until > NOW())) AS suspended, \
        u.banned_until, u.ban_reason, u.account_type, \
        COALESCE(u.last_active_at, u.created_on) AS last_active, \
        u.default_storage_bytes AS base_quota, \
        (u.default_storage_bytes + COALESCE(sub.extra, 0))::BIGINT AS quota, \
        COALESCE(used.used, 0)::BIGINT AS used, \
        COALESCE(fs.files, 0)::BIGINT AS files, \
        COALESCE(fs.public_links, 0)::BIGINT AS public_links, \
        COALESCE(fs.enc_bytes, 0)::BIGINT AS enc_bytes, \
        COALESCE(fs.bytes, 0)::BIGINT AS bytes, \
        COALESCE(req.request_links, 0)::BIGINT AS request_links, \
        sub.plan AS plan, sub.renews AS renews, (sub.created_by IS NOT NULL) AS paying \
    FROM users u \
    LEFT JOIN used ON used.user_id = u.id \
    LEFT JOIN fs ON fs.user_id = u.id \
    LEFT JOIN sub ON sub.created_by = u.id \
    LEFT JOIN req ON req.owner_user_id = u.id)";

/// The console's filters, as SQL over `base`. Fixed strings, never user input.
fn filter_sql(f: &str) -> Option<&'static str> {
    Some(match f {
        "all" => "TRUE",
        "paying" => "paying",
        "quota" => "quota > 0 AND used >= quota * 0.9",
        "public" => "public_links > 0",
        "request" => "request_links > 0",
        "unverified" => "NOT email_verified",
        "inactive" => "last_active < NOW() - INTERVAL '30 days'",
        "suspended" => "suspended",
        _ => return None,
    })
}
const FILTERS: [&str; 8] = ["all", "paying", "quota", "public", "request", "unverified", "inactive", "suspended"];

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_limit")]
    limit: i64,
    #[serde(default)]
    offset: i64,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    filter: Option<String>,
    /// active (default), joined, used, files
    #[serde(default)]
    sort: Option<String>,
}
fn default_limit() -> i64 {
    60
}

fn row_json(r: &sqlx::postgres::PgRow) -> serde_json::Value {
    json!({
        "id": r.get::<String, _>("id"),
        "username": r.get::<String, _>("username"),
        "email": r.get::<String, _>("email"),
        "created_on": r.get::<chrono::DateTime<chrono::Utc>, _>("created_on"),
        "email_verified": r.get::<bool, _>("email_verified"),
        "restricted": r.get::<bool, _>("is_restricted"),
        "suspended": r.try_get::<Option<bool>, _>("suspended").ok().flatten().unwrap_or(false),
        "suspended_until": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("banned_until").ok().flatten(),
        "suspension_reason": r.try_get::<Option<String>, _>("ban_reason").ok().flatten(),
        "account_type": r.get::<String, _>("account_type"),
        "last_active": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_active").ok().flatten(),
        "base_quota": r.get::<i64, _>("base_quota"),
        "quota": r.get::<i64, _>("quota"),
        "used": r.get::<i64, _>("used"),
        "files": r.get::<i64, _>("files"),
        "public_links": r.get::<i64, _>("public_links"),
        "enc_bytes": r.get::<i64, _>("enc_bytes"),
        "bytes": r.get::<i64, _>("bytes"),
        "request_links": r.get::<i64, _>("request_links"),
        "plan": r.try_get::<Option<String>, _>("plan").ok().flatten(),
        "renews": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("renews").ok().flatten(),
        "paying": r.try_get::<Option<bool>, _>("paying").ok().flatten().unwrap_or(false),
    })
}

/// A case-insensitive substring of username or email, or an exact id. Both
/// placeholders bound NULL means no search.
const SEARCH: &str = "($1::text IS NULL OR username ILIKE $1 ESCAPE '\\' OR email ILIKE $1 ESCAPE '\\' OR id = $2)";

async fn list(State(state): State<crate::AppState>, Query(p): Query<ListParams>) -> impl IntoResponse {
    let limit = p.limit.clamp(1, 500);
    let offset = p.offset.max(0);
    let filter = p.filter.as_deref().unwrap_or("all");
    let Some(fsql) = filter_sql(filter) else {
        return respond(400, "Unknown filter", vec![], json!({}));
    };
    let order = match p.sort.as_deref().unwrap_or("active") {
        "joined" => "created_on DESC",
        "used" => "used DESC",
        "files" => "files DESC",
        _ => "last_active DESC NULLS LAST",
    };
    let q = p.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    let pattern = q.as_ref().map(|s| format!("%{}%", s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")));

    let rows = sqlx::query(&format!(
        "{ACCOUNTS} SELECT * FROM base WHERE {SEARCH} AND ({fsql}) ORDER BY {order}, id LIMIT $3 OFFSET $4"
    ))
    .bind(&pattern)
    .bind(&q)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg_pool)
    .await;

    // Every filter's count under the current search, for the chips.
    let counts_sql = FILTERS
        .iter()
        .map(|f| format!("COUNT(*) FILTER (WHERE {})::BIGINT AS \"{f}\"", filter_sql(f).unwrap_or("FALSE")))
        .collect::<Vec<_>>()
        .join(", ");
    let counts = sqlx::query(&format!("{ACCOUNTS} SELECT {counts_sql} FROM base WHERE {SEARCH}"))
        .bind(&pattern)
        .bind(&q)
        .fetch_one(&state.pg_pool)
        .await;

    match (rows, counts) {
        (Ok(rows), Ok(c)) => {
            let counts: serde_json::Map<String, serde_json::Value> =
                FILTERS.iter().map(|f| (f.to_string(), json!(c.try_get::<i64, _>(*f).unwrap_or(0)))).collect();
            let total = counts.get(filter).cloned().unwrap_or(json!(0));
            respond(
                200,
                "Accounts",
                vec![],
                json!({
                    "accounts": rows.iter().map(row_json).collect::<Vec<_>>(),
                    "counts": counts,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                    "q": q,
                    "filter": filter,
                }),
            )
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("[admin] accounts list: {e:?}");
            respond(500, "Failed to list accounts", vec![], json!({}))
        }
    }
}

async fn detail(State(state): State<crate::AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let row = sqlx::query(&format!("{ACCOUNTS} SELECT * FROM base WHERE id = $1"))
        .bind(&id)
        .fetch_optional(&state.pg_pool)
        .await;
    let row = match row {
        Ok(Some(r)) => r,
        Ok(None) => return respond(404, "Account not found", vec![], json!({})),
        Err(e) => {
            eprintln!("[admin] account detail: {e:?}");
            return respond(500, "Failed to load account", vec![], json!({}));
        }
    };

    let invoices = sqlx::query(
        "SELECT invoice_number, reference_id, amount, currency, status, subscription_name, subscription_cycle, created_on \
         FROM orders WHERE user_id = $1 ORDER BY created_on DESC LIMIT 24",
    )
    .bind(&id)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_default()
    .iter()
    .map(|r| {
        json!({
            "invoice": r.try_get::<Option<String>, _>("invoice_number").ok().flatten(),
            "reference": r.try_get::<Option<String>, _>("reference_id").ok().flatten(),
            "amount": r.try_get::<Option<i64>, _>("amount").ok().flatten(),
            "currency": r.try_get::<Option<String>, _>("currency").ok().flatten(),
            "status": r.try_get::<Option<String>, _>("status").ok().flatten(),
            "plan": r.try_get::<Option<String>, _>("subscription_name").ok().flatten(),
            "cycle": r.try_get::<Option<String>, _>("subscription_cycle").ok().flatten(),
            "created_on": r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("created_on").ok().flatten(),
        })
    })
    .collect::<Vec<_>>();

    // Bytes served through this account's share links since the 1st of the month:
    // every recorded link open counted at the size of the file it opened.
    let out_month = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT COALESCE(SUM(f.size), 0)::BIGINT FROM share_access_events e JOIN files f ON f.id = e.file_id \
         WHERE e.owner_user_id = $1 AND e.created_at >= date_trunc('month', NOW())",
    )
    .bind(&id)
    .fetch_one(&state.pg_pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0);

    let mut account = row_json(&row);
    account["out_month"] = json!(out_month);
    respond(200, "Account", vec![], json!({ "account": account, "invoices": invoices }))
}

#[derive(Deserialize)]
pub struct QuotaInput {
    /// Bytes to add to the base quota; negative lowers it. The result never
    /// drops below zero.
    add_bytes: i64,
}

async fn quota(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
    Json(body): Json<QuotaInput>,
) -> impl IntoResponse {
    if body.add_bytes == 0 {
        return respond(400, "Nothing to change", vec![], json!({}));
    }
    let r = sqlx::query_scalar::<_, i64>(
        "UPDATE users SET default_storage_bytes = GREATEST(0, default_storage_bytes + $1) WHERE id = $2 \
         RETURNING default_storage_bytes",
    )
    .bind(body.add_bytes)
    .bind(&id)
    .fetch_optional(&state.pg_pool)
    .await;
    match r {
        Ok(Some(b)) => respond(200, "Quota changed", vec![], json!({ "base_quota": b })),
        Ok(None) => respond(404, "Account not found", vec![], json!({})),
        Err(_) => respond(500, "Failed to change quota", vec![], json!({})),
    }
}

/// Every file and folder share this account has goes off, the same way the
/// owner turns one off: link type off, token dropped (so an old URL cannot come
/// back if sharing is re-enabled), and `public_access` cleared so the id path
/// stops serving too. Files stay stored.
async fn disable_links(State(state): State<crate::AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let mut tx = match state.pg_pool.begin().await {
        Ok(t) => t,
        Err(_) => return respond(500, "Database error", vec![], json!({})),
    };
    let files = sqlx::query(
        "UPDATE files SET share_type = 'off', share_token = NULL, public_access = false \
         WHERE user_id = $1 AND (share_token IS NOT NULL OR public_access)",
    )
    .bind(&id)
    .execute(&mut *tx)
    .await;
    let folders = sqlx::query(
        "UPDATE folders SET share_type = 'off', share_token = NULL, public_access = false \
         WHERE user_id = $1 AND (share_token IS NOT NULL OR public_access)",
    )
    .bind(&id)
    .execute(&mut *tx)
    .await;
    match (files, folders) {
        (Ok(f), Ok(d)) => {
            if tx.commit().await.is_err() {
                return respond(500, "Failed to disable links", vec![], json!({}));
            }
            respond(
                200,
                "Links disabled",
                vec![],
                json!({ "files": f.rows_affected(), "folders": d.rows_affected() }),
            )
        }
        _ => respond(500, "Failed to disable links", vec![], json!({})),
    }
}
