//! GET /admin/traction - SiloCat's growth from its own tables, whole history.
//!
//! Signups per week, Plus subscriptions started per week and live now, orders
//! per month, bytes uploaded per week (files still stored), and anonymous
//! sessions started per week. Anonymous drops themselves are deleted by
//! watchcat after their TTL, so the sessions behind them are what keeps a
//! history. Active accounts come from users.last_active_at (0046).

use axum::{extract::State, response::IntoResponse, routing::get, Router};
use serde_json::json;
use sqlx::Row;

use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", get(handle))
}

async fn handle(State(state): State<crate::AppState>) -> impl IntoResponse {
    let pool = &state.pg_pool;
    let signups = sqlx::query(
        "SELECT date_trunc('week', created_on)::date AS wk, COUNT(*)::BIGINT AS n, \
                COUNT(*) FILTER (WHERE email_verified)::BIGINT AS verified \
         FROM users GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    let subs = sqlx::query(
        "SELECT date_trunc('week', created_on)::date AS wk, COUNT(*)::BIGINT AS n \
         FROM subscriptions GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    let orders = sqlx::query(
        "SELECT date_trunc('month', created_on)::date AS mo, currency, \
                SUM(amount) FILTER (WHERE status IN ('completed','paid'))::BIGINT AS amount, \
                COUNT(*) FILTER (WHERE status IN ('completed','paid'))::BIGINT AS paid \
         FROM orders GROUP BY 1, 2 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    let uploads = sqlx::query(
        "SELECT date_trunc('week', created_on)::date AS wk, COUNT(*)::BIGINT AS files, \
                COALESCE(SUM(size), 0)::BIGINT AS bytes, \
                COUNT(*) FILTER (WHERE encrypted)::BIGINT AS encrypted \
         FROM files WHERE deleted = false AND user_id IS NOT NULL GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    let anon = sqlx::query(
        "SELECT date_trunc('week', created_at)::date AS wk, COUNT(*)::BIGINT AS n \
         FROM anonymous_users GROUP BY 1 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    // Signups by where they came from (migration 0047), and how many of each
    // went on to pay. Only accounts that carry a source: the rest predate it.
    let sources = sqlx::query(
        "SELECT date_trunc('week', u.created_on)::date AS wk, u.signup_source AS src, COUNT(*)::BIGINT AS n, \
                COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM subscriptions s WHERE s.created_by = u.id))::BIGINT AS paid \
         FROM users u WHERE u.signup_source IS NOT NULL GROUP BY 1, 2 ORDER BY 1",
    )
    .fetch_all(pool)
    .await;
    let now = sqlx::query(
        "SELECT \
            (SELECT COUNT(*) FROM users)::BIGINT AS accounts, \
            (SELECT COUNT(*) FROM users WHERE last_active_at > NOW() - INTERVAL '7 days')::BIGINT AS active7, \
            (SELECT COUNT(*) FROM users WHERE last_active_at > NOW() - INTERVAL '30 days')::BIGINT AS active30, \
            (SELECT COUNT(DISTINCT created_by) FROM subscriptions WHERE expires_on > NOW())::BIGINT AS paying",
    )
    .fetch_one(pool)
    .await;

    let wk = |rows: &Vec<sqlx::postgres::PgRow>, cols: &[&str]| -> Vec<serde_json::Value> {
        rows.iter()
            .map(|r| {
                let mut o = serde_json::Map::new();
                o.insert("week".into(), json!(r.get::<chrono::NaiveDate, _>("wk")));
                for c in cols {
                    o.insert((*c).into(), json!(r.try_get::<i64, _>(*c).unwrap_or(0)));
                }
                serde_json::Value::Object(o)
            })
            .collect()
    };

    match (signups, subs, orders, uploads, anon, now, sources) {
        (Ok(s), Ok(b), Ok(o), Ok(u), Ok(a), Ok(n), Ok(src)) => respond(
            200,
            "Traction",
            vec![],
            json!({
                "signups": wk(&s, &["n", "verified"]),
                "subscriptions": wk(&b, &["n"]),
                "uploads": wk(&u, &["files", "bytes", "encrypted"]),
                "anon_sessions": wk(&a, &["n"]),
                "sources": src.iter().map(|r| json!({
                    "week": r.get::<chrono::NaiveDate, _>("wk"),
                    "source": r.get::<String, _>("src"),
                    "n": r.get::<i64, _>("n"),
                    "paid": r.get::<i64, _>("paid"),
                })).collect::<Vec<_>>(),
                "orders": o.iter().map(|r| json!({
                    "month": r.get::<chrono::NaiveDate, _>("mo"),
                    "currency": r.try_get::<Option<String>, _>("currency").ok().flatten(),
                    "amount": r.try_get::<Option<i64>, _>("amount").ok().flatten().unwrap_or(0),
                    "paid": r.try_get::<Option<i64>, _>("paid").ok().flatten().unwrap_or(0),
                })).collect::<Vec<_>>(),
                "now": {
                    "accounts": n.get::<i64, _>("accounts"),
                    "active7": n.get::<i64, _>("active7"),
                    "active30": n.get::<i64, _>("active30"),
                    "paying": n.get::<i64, _>("paying"),
                },
            }),
        ),
        _ => respond(500, "Failed to compute traction", vec![], json!({})),
    }
}
