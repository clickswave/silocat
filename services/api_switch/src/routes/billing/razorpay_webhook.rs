//! Razorpay webhook (silocat-suggestions.md P0/P1 #13).
//!
//! Reconciles payment lifecycle events server-side so billing state is correct
//! even when the browser never completes the /billing/verify handshake:
//!   * order.paid / payment.captured -> grant benefits for a still-pending order
//!   * payment.failed                -> mark the order failed
//!   * refund.*                      -> mark refunded and reverse the benefit
//!
//! This endpoint is mounted OUTSIDE the X-Authority-Sign gate (Razorpay calls it
//! directly); its authenticity comes from the `X-Razorpay-Signature` HMAC over
//! the raw body, keyed by RAZORPAY_WEBHOOK_SECRET. It fails closed if that secret
//! is unset.

use axum::{body::Bytes, extract::State, http::HeaderMap, response::IntoResponse};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;

use crate::routes::respond;
use sqlx::Row;

/// Paid plan → (display name, storage bytes). Must match order.rs / verify.rs.
fn plan_grant(identifier: &str) -> Option<(&'static str, i64)> {
    match identifier {
        "plus" => Some(("Plus", 200 * 1024 * 1024 * 1024)),
        "pro" => Some(("Pro", 2 * 1024_i64.pow(4))),
        _ => None,
    }
}

fn webhook_secret() -> Option<String> {
    std::env::var("RAZORPAY_WEBHOOK_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn razorpay_order_id(event: &Value) -> Option<String> {
    event
        .pointer("/payload/order/entity/id")
        .and_then(|v| v.as_str())
        .or_else(|| event.pointer("/payload/payment/entity/order_id").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

pub async fn handle(
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    // Fail closed when not configured.
    let secret = match webhook_secret() {
        Some(s) => s,
        None => return respond(503, "Webhook not configured", vec![], json!({})),
    };

    // Authenticate: HMAC-SHA256 over the exact raw body (constant-time compare).
    let sig_hex = headers
        .get("X-Razorpay-Signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).expect("HMAC any key");
    mac.update(&body);
    let provided = match hex::decode(sig_hex) {
        Ok(b) => b,
        Err(_) => return respond(400, "Invalid signature", vec![], json!({})),
    };
    if mac.verify_slice(&provided).is_err() {
        return respond(400, "Invalid signature", vec![], json!({}));
    }

    let event: Value = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(_) => return respond(400, "Invalid payload", vec![], json!({})),
    };
    let event_type = event.get("event").and_then(|e| e.as_str()).unwrap_or("");

    match event_type {
        "order.paid" | "payment.captured" => {
            if let Some(oid) = razorpay_order_id(&event) {
                if let Err(e) = reconcile_and_grant(&state, &oid).await {
                    eprintln!("[razorpay-webhook] grant failed for {}: {:?}", oid, e);
                }
            }
        }
        "payment.failed" => {
            if let Some(oid) = razorpay_order_id(&event) {
                let _ = sqlx::query(
                    "UPDATE orders SET status = 'failed' WHERE reference_id = $1 AND status = 'pending'",
                )
                .bind(&oid)
                .execute(&state.pg_pool)
                .await;
            }
        }
        "refund.created" | "refund.processed" | "payment.refunded" => {
            if let Some(oid) = razorpay_order_id(&event) {
                if let Err(e) = revoke_for_order(&state, &oid).await {
                    eprintln!("[razorpay-webhook] revoke failed for {}: {:?}", oid, e);
                }
            }
        }
        _ => {}
    }

    // Ack a validly-signed event so Razorpay stops retrying.
    respond(200, "ok", vec![], json!({}))
}

/// Grant an order's benefits if it is still pending (idempotent, atomic). Same
/// grant shape as /billing/verify: the conditional claim serialises with it.
async fn reconcile_and_grant(state: &crate::AppState, order_id: &str) -> anyhow::Result<()> {
    let order = sqlx::query!(
        "SELECT user_id, additional_space, subscription_name, subscription_cycle, status FROM orders WHERE reference_id = $1",
        order_id
    )
    .fetch_optional(&state.pg_pool)
    .await?;
    let order = match order {
        Some(o) if o.status == "pending" => o,
        _ => return Ok(()), // already completed/failed/unknown: nothing to do
    };

    let mut tx = state.pg_pool.begin().await?;
    let claimed = sqlx::query(
        "UPDATE orders SET status = 'completed' WHERE reference_id = $1 AND status = 'pending' RETURNING reference_id",
    )
    .bind(order_id)
    .fetch_optional(&mut *tx)
    .await?;
    if claimed.is_none() {
        tx.rollback().await?;
        return Ok(());
    }

    if order.additional_space > 0 {
        sqlx::query("UPDATE users SET default_storage_bytes = default_storage_bytes + $1 WHERE id = $2")
            .bind(order.additional_space)
            .bind(&order.user_id)
            .execute(&mut *tx)
            .await?;
    } else if let Some((plan_display, plan_space)) = plan_grant(&order.subscription_name) {
        let interval = if order.subscription_cycle == "annual" { "1 year" } else { "1 month" };
        let row = sqlx::query(
            &format!(
                "INSERT INTO subscriptions (name, additional_space, created_by, expires_on) \
                 VALUES ($1, $2, $3, NOW() + INTERVAL '{}') RETURNING id",
                interval
            )
        )
        .bind(plan_display)
        .bind(plan_space)
        .bind(&order.user_id)
        .fetch_one(&mut *tx)
        .await?;
        let sub_id: String = row.get("id");
        sqlx::query("UPDATE users SET subscription_id = $1 WHERE id = $2")
            .bind(sub_id)
            .bind(&order.user_id)
            .execute(&mut *tx)
            .await?;
    }

    tx.commit().await?;
    Ok(())
}

/// Reverse an order's benefit on refund: mark refunded, and either subtract the
/// purchased storage (floored at 0) or expire the user's Pro subscription now.
async fn revoke_for_order(state: &crate::AppState, order_id: &str) -> anyhow::Result<()> {
    let order = sqlx::query!(
        "SELECT user_id, additional_space, subscription_name FROM orders WHERE reference_id = $1",
        order_id
    )
    .fetch_optional(&state.pg_pool)
    .await?;
    let order = match order {
        Some(o) => o,
        None => return Ok(()),
    };

    let mut tx = state.pg_pool.begin().await?;
    sqlx::query("UPDATE orders SET status = 'refunded' WHERE reference_id = $1")
        .bind(order_id)
        .execute(&mut *tx)
        .await?;

    if order.additional_space > 0 {
        sqlx::query("UPDATE users SET default_storage_bytes = GREATEST(default_storage_bytes - $1, 0) WHERE id = $2")
            .bind(order.additional_space)
            .bind(&order.user_id)
            .execute(&mut *tx)
            .await?;
    } else if let Some((plan_display, _)) = plan_grant(&order.subscription_name) {
        // Expire this plan's subscription for the user immediately.
        sqlx::query(
            "UPDATE subscriptions SET expires_on = NOW() WHERE created_by = $1 AND name = $2 AND expires_on > NOW()",
        )
        .bind(&order.user_id)
        .bind(plan_display)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}
