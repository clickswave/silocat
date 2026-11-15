//! Razorpay webhook (silocat-suggestions.md P0/P1 #13).
//!
//! Reconciles payment lifecycle events server-side so billing state is correct
//! even when the browser never completes the /billing/verify handshake:
//!   * order.paid / payment.captured -> grant benefits for a still-pending order
//!   * payment.failed                -> mark the order failed
//!   * refund.*, payment.refunded    -> once the payment is fully refunded,
//!                                      mark it refunded and reverse the benefit
//!   * payment.dispute.lost          -> the same, for a chargeback
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

/// Paid plan → (display name, storage bytes). Canonical source: libs::plans.
fn plan_grant(identifier: &str) -> Option<(&'static str, i64)> {
    crate::libs::plans::plan_grant(identifier)
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

/// The amount + currency Razorpay actually settled, from either the order or the
/// payment entity on the event.
fn razorpay_paid_amount(event: &Value) -> Option<(i64, String)> {
    let entity = event
        .pointer("/payload/order/entity")
        .or_else(|| event.pointer("/payload/payment/entity"))?;
    let amount = entity.get("amount").and_then(|v| v.as_i64())?;
    let currency = entity.get("currency").and_then(|v| v.as_str())?.to_string();
    Some((amount, currency))
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
                if let Err(e) = reconcile_and_grant(&state, &oid, razorpay_paid_amount(&event)).await {
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
        "refund.created" | "refund.processed" | "payment.refunded" | "payment.dispute.lost" => {
            if let Err(e) = revoke_for_payment(&state, &event, event_type == "payment.dispute.lost").await {
                eprintln!("[razorpay-webhook] revoke failed for {}: {:?}", event_type, e);
            }
        }
        "subscription.charged" => {
            if let Err(e) = handle_subscription_charged(&state, &event).await {
                eprintln!("[razorpay-webhook] subscription.charged failed: {:?}", e);
            }
        }
        "subscription.cancelled" | "subscription.halted" | "subscription.completed" => {
            // The mandate stopped: we just stop extending. The user keeps access
            // until the current expires_on, then the plan lapses naturally (WatchCat
            // nulls the pointer and quota reclaims). No immediate action needed.
        }
        _ => {}
    }

    // Ack a validly-signed event so Razorpay stops retrying.
    respond(200, "ok", vec![], json!({}))
}

/// Grant an order's benefits if it is still pending (idempotent, atomic). Same
/// grant shape as /billing/verify: the conditional claim serialises with it.
async fn reconcile_and_grant(
    state: &crate::AppState,
    order_id: &str,
    paid: Option<(i64, String)>,
) -> anyhow::Result<()> {
    let order = sqlx::query!(
        "SELECT user_id, additional_space, subscription_name, subscription_cycle, status, amount, currency FROM orders WHERE reference_id = $1",
        order_id
    )
    .fetch_optional(&state.pg_pool)
    .await?;
    let order = match order {
        Some(o) if o.status == "pending" => o,
        _ => return Ok(()), // already completed/failed/unknown: nothing to do
    };

    // Defense in depth on top of the HMAC: grant only if what Razorpay actually
    // captured meets the order's expected amount and currency, so a partial
    // capture or a replayed event for a different order can't unlock a plan.
    if let Some((paid_amt, paid_cur)) = paid {
        if paid_amt < order.amount || !paid_cur.eq_ignore_ascii_case(&order.currency) {
            eprintln!(
                "[razorpay-webhook] amount/currency mismatch for {}: captured {} {} vs order {} {}; not granting",
                order_id, paid_amt, paid_cur, order.amount, order.currency
            );
            return Ok(());
        }
    }

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
        // Replace any active paid plan first so renewals/upgrades don't stack quota
        // (see the same guard in verify.rs). Promo/invite grants are left alone.
        sqlx::query(
            "UPDATE subscriptions SET expires_on = NOW() \
             WHERE created_by = $1 AND invited = false AND expires_on > NOW()"
        )
        .bind(&order.user_id)
        .execute(&mut *tx)
        .await?;
        // A Razorpay subscription id ("sub_...") as the reference means this grant
        // came from a recurring subscription: link the row so subscription.charged
        // renewals can extend it.
        let rzp_sub = if order_id.starts_with("sub_") { Some(order_id.to_string()) } else { None };
        let row = sqlx::query(
            &format!(
                "INSERT INTO subscriptions (name, additional_space, created_by, expires_on, razorpay_subscription_id) \
                 VALUES ($1, $2, $3, NOW() + INTERVAL '{}', $4) RETURNING id",
                interval
            )
        )
        .bind(plan_display)
        .bind(plan_space)
        .bind(&order.user_id)
        .bind(rzp_sub)
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

/// Extend a subscription's expiry by one billing cycle (on a renewal charge).
async fn extend_subscription(state: &crate::AppState, rzp_sub_id: &str) -> anyhow::Result<()> {
    let cycle: Option<String> = sqlx::query_scalar(
        "SELECT subscription_cycle FROM orders WHERE reference_id = $1 ORDER BY created_on DESC LIMIT 1",
    )
    .bind(rzp_sub_id)
    .fetch_optional(&state.pg_pool)
    .await?;
    let interval = if cycle.as_deref() == Some("annual") { "1 year" } else { "1 month" };
    // Extend from the later of NOW and the current expiry, so a slightly-early
    // renewal charge never shortens the paid period.
    sqlx::query(&format!(
        "UPDATE subscriptions SET expires_on = GREATEST(expires_on, NOW()) + INTERVAL '{}' \
         WHERE razorpay_subscription_id = $1",
        interval
    ))
    .bind(rzp_sub_id)
    .execute(&state.pg_pool)
    .await?;
    Ok(())
}

/// A subscription was charged. Period 1 is granted by /billing/verify; this
/// EXTENDS on later renewal charges, and also acts as the server-side safety net
/// that grants period 1 if the browser never reached /verify.
async fn handle_subscription_charged(state: &crate::AppState, event: &Value) -> anyhow::Result<()> {
    let sub_id = match event.pointer("/payload/subscription/entity/id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return Ok(()),
    };
    // Note the payment on the subscription's order, so a refund of it, which
    // carries the order of Razorpay's invoice rather than ours, finds it.
    if let Some(payment_id) = event.pointer("/payload/payment/entity/id").and_then(|v| v.as_str()) {
        sqlx::query(
            "UPDATE orders SET transactions = array_append(transactions, $2) \
             WHERE reference_id = $1 \
               AND NOT EXISTS (SELECT 1 FROM unnest(transactions) t WHERE t->>'payment_id' = $3)",
        )
        .bind(sub_id)
        .bind(json!({ "payment_id": payment_id, "event": "subscription.charged" }))
        .bind(payment_id)
        .execute(&state.pg_pool)
        .await?;
    }

    let paid_count = event
        .pointer("/payload/subscription/entity/paid_count")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    let existing: Option<String> = sqlx::query_scalar(
        "SELECT id FROM subscriptions WHERE razorpay_subscription_id = $1 LIMIT 1",
    )
    .bind(sub_id)
    .fetch_optional(&state.pg_pool)
    .await?;

    if existing.is_some() {
        // Already granted period 1 (verify.rs). Extend only on later charges so a
        // duplicated first-charge event can't double-count the period.
        if paid_count > 1 {
            extend_subscription(state, sub_id).await?;
        }
        return Ok(());
    }

    // No local row yet: grant period 1 from the pending order (safety net for a
    // user who paid then closed the tab before /verify ran).
    reconcile_and_grant(state, sub_id, None).await?;
    Ok(())
}

/// Reverse what a payment bought once it is fully refunded or its dispute is
/// lost. A partial refund leaves the grant alone. One refund arrives as up to
/// three events (refund.created, refund.processed, payment.refunded), so each
/// payment is reversed at most once: the order records the refunded payment
/// id in the same statement that marks it refunded.
async fn revoke_for_payment(state: &crate::AppState, event: &Value, chargeback: bool) -> anyhow::Result<()> {
    let payment = match event.pointer("/payload/payment/entity") {
        Some(p) => p,
        None => return Ok(()),
    };
    let payment_id = match payment.get("id").and_then(|v| v.as_str()) {
        Some(id) => id,
        None => return Ok(()),
    };
    let amount = payment.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
    let refunded = payment.get("amount_refunded").and_then(|v| v.as_i64()).unwrap_or(0);
    let full = payment.get("status").and_then(|v| v.as_str()) == Some("refunded")
        || payment.get("refund_status").and_then(|v| v.as_str()) == Some("full")
        || (amount > 0 && refunded >= amount);
    if !full && !chargeback {
        eprintln!("[razorpay-webhook] partial refund on {}: grant left in place", payment_id);
        return Ok(());
    }
    let order_id = payment.get("order_id").and_then(|v| v.as_str()).unwrap_or("");
    let rzp_sub = payment.get("subscription_id").and_then(|v| v.as_str()).unwrap_or("");

    let mut tx = state.pg_pool.begin().await?;
    // A one-time order is ours by its order id. A subscription payment carries
    // the order of Razorpay's invoice, so it is found by the payment id that
    // /billing/verify or subscription.charged noted on the subscription's order.
    let order = sqlx::query_as::<_, (String, String, i64, String)>(
        "SELECT reference_id, user_id, additional_space, subscription_name FROM orders \
         WHERE reference_id = $1 OR reference_id = $2 \
            OR EXISTS (SELECT 1 FROM unnest(transactions) t WHERE t->>'payment_id' = $3) \
         ORDER BY created_on DESC LIMIT 1 FOR UPDATE",
    )
    .bind(order_id)
    .bind(rzp_sub)
    .bind(payment_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (reference, user_id, additional_space, plan) = match order {
        Some(o) => o,
        None => return Ok(()),
    };
    let is_subscription = reference.starts_with("sub_");

    // A one-time order is paid once, so it is reversed only from completed. A
    // subscription is paid every cycle, so another payment of one already
    // marked refunded can still be reversed.
    let claimed = sqlx::query(
        "UPDATE orders SET status = 'refunded', transactions = array_append(transactions, $2) \
         WHERE reference_id = $1 \
           AND (status = 'completed' OR (status = 'refunded' AND $3)) \
           AND NOT ($2 = ANY(transactions)) \
         RETURNING reference_id",
    )
    .bind(&reference)
    .bind(json!({ "refunded_payment_id": payment_id }))
    .bind(is_subscription)
    .fetch_optional(&mut *tx)
    .await?;
    if claimed.is_none() {
        tx.rollback().await?;
        return Ok(());
    }

    if additional_space > 0 {
        sqlx::query("UPDATE users SET default_storage_bytes = GREATEST(default_storage_bytes - $1, 0) WHERE id = $2")
            .bind(additional_space)
            .bind(&user_id)
            .execute(&mut *tx)
            .await?;
    } else if is_subscription {
        sqlx::query(
            "UPDATE subscriptions SET expires_on = NOW() WHERE razorpay_subscription_id = $1 AND expires_on > NOW()",
        )
        .bind(&reference)
        .execute(&mut *tx)
        .await?;
    } else if let Some((plan_display, _)) = plan_grant(&plan) {
        // The plan this order bought: a paid row, never an invite or promo grant.
        sqlx::query(
            "UPDATE subscriptions SET expires_on = NOW() \
             WHERE created_by = $1 AND name = $2 AND invited = FALSE \
               AND razorpay_subscription_id IS NULL AND expires_on > NOW()",
        )
        .bind(&user_id)
        .bind(plan_display)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    //! These send signed webhook events for accounts in a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Each deletes the
    //! account and the orders it made.

    use super::*;
    use crate::test_support::{account, forget, state, SETTLING_ORDERS};
    use axum::http::HeaderValue;

    const SECRET: &str = "webhook-secret-for-tests";
    const TIB: i64 = crate::libs::plans::TIB;

    async fn send(st: &crate::AppState, event: Value) -> u16 {
        unsafe { std::env::set_var("RAZORPAY_WEBHOOK_SECRET", SECRET) };
        let body = serde_json::to_vec(&event).unwrap();
        let mut mac = Hmac::<Sha256>::new_from_slice(SECRET.as_bytes()).unwrap();
        mac.update(&body);
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Razorpay-Signature",
            HeaderValue::from_str(&hex::encode(mac.finalize().into_bytes())).unwrap(),
        );
        handle(State(st.clone()), headers, Bytes::from(body)).await.into_response().status().as_u16()
    }

    /// A payment as Razorpay puts it on refund and dispute events.
    fn payment(id: &str, order_id: &str, amount: i64, refunded: i64) -> Value {
        json!({
            "id": id,
            "entity": "payment",
            "amount": amount,
            "currency": "USD",
            "order_id": order_id,
            "amount_refunded": refunded,
            "refund_status": if refunded >= amount { "full" } else if refunded > 0 { "partial" } else { "null" },
            "status": if refunded >= amount { "refunded" } else { "captured" },
        })
    }

    fn refund_event(kind: &str, pay: Value) -> Value {
        json!({
            "event": kind,
            "payload": {
                "refund": { "entity": { "id": "rfnd_1", "payment_id": pay["id"], "amount": pay["amount_refunded"] } },
                "payment": { "entity": pay },
            }
        })
    }

    async fn completed_order(st: &crate::AppState, user: &str, reference: &str, plan: &str, space: i64, txs: Vec<Value>) {
        sqlx::query(
            "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, \
             payment_gateway, currency, amount, status, details, transactions) \
             VALUES ($1, $2, $3, 'monthly', $4, 'razorpay', 'USD', 1000, 'completed', '{}', $5)",
        )
        .bind(reference)
        .bind(user)
        .bind(plan)
        .bind(space)
        .bind(txs)
        .execute(&st.pg_pool)
        .await
        .unwrap();
    }

    async fn grant(st: &crate::AppState, user: &str, name: &str, invited: bool, rzp_sub: Option<&str>) -> String {
        sqlx::query_scalar(
            "INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited, razorpay_subscription_id) \
             VALUES ($1, $2, $3, NOW() + INTERVAL '30 days', $4, $5) RETURNING id",
        )
        .bind(name)
        .bind(2 * TIB)
        .bind(user)
        .bind(invited)
        .bind(rzp_sub)
        .fetch_one(&st.pg_pool)
        .await
        .unwrap()
    }

    async fn live(st: &crate::AppState, sub: &str) -> bool {
        sqlx::query_scalar("SELECT expires_on > NOW() FROM subscriptions WHERE id = $1")
            .bind(sub)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn storage(st: &crate::AppState, id: &str) -> i64 {
        sqlx::query_scalar("SELECT default_storage_bytes FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn teardown(st: &crate::AppState, user: &str) {
        sqlx::query("DELETE FROM orders WHERE user_id = $1").bind(user).execute(&st.pg_pool).await.unwrap();
        forget(&st.pg_pool, user).await;
    }

    fn rid(prefix: &str) -> String {
        format!("{}_{}", prefix, crate::libs::rng::string(14))
    }

    #[tokio::test]
    async fn one_refund_takes_a_storage_purchase_back_once() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let base = storage(&st, &user).await;
        // A storage add-on bought before they stopped being sold, on top of a
        // bigger invite grant.
        sqlx::query("UPDATE users SET default_storage_bytes = $1 WHERE id = $2")
            .bind(3 * TIB + base)
            .bind(&user)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        let order = rid("order");
        completed_order(&st, &user, &order, "", TIB, vec![]).await;
        let pay = payment(&rid("pay"), &order, 1000, 1000);
        for kind in ["refund.created", "refund.processed", "payment.refunded"] {
            assert_eq!(send(&st, refund_event(kind, pay.clone())).await, 200);
        }
        let after = storage(&st, &user).await;
        teardown(&st, &user).await;
        assert_eq!(after, 2 * TIB + base, "the add-on comes off once, and nothing else does");
    }

    #[tokio::test]
    async fn a_partial_refund_leaves_the_plan_in_place() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let order = rid("order");
        completed_order(&st, &user, &order, "pro", 0, vec![]).await;
        let sub = grant(&st, &user, "Pro", false, None).await;
        let pay = payment(&rid("pay"), &order, 1000, 300);
        send(&st, refund_event("refund.processed", pay)).await;
        let kept = live(&st, &sub).await;
        teardown(&st, &user).await;
        assert!(kept, "a partial refund must not end the whole plan");
    }

    #[tokio::test]
    async fn a_full_refund_ends_the_paid_plan_but_not_an_invite_grant() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let order = rid("order");
        completed_order(&st, &user, &order, "pro", 0, vec![]).await;
        let invite = grant(&st, &user, "Pro", true, None).await;
        let paid = grant(&st, &user, "Pro", false, None).await;
        let pay = payment(&rid("pay"), &order, 1000, 1000);
        send(&st, refund_event("payment.refunded", pay)).await;
        let (invite_live, paid_live) = (live(&st, &invite).await, live(&st, &paid).await);
        teardown(&st, &user).await;
        assert!(!paid_live, "the refunded plan ends");
        assert!(invite_live, "an invite grant was not paid for and is not refunded");
    }

    #[tokio::test]
    async fn refunding_a_subscription_payment_ends_that_subscription() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let rzp_sub = rid("sub");
        let first = rid("pay");
        // /billing/verify records the first payment on the order.
        completed_order(&st, &user, &rzp_sub, "pro", 0, vec![json!({ "payment_id": first, "signature": "x" })]).await;
        let sub = grant(&st, &user, "Pro", false, Some(&rzp_sub)).await;
        // A subscription payment carries the order of its invoice, not ours.
        let pay = payment(&first, &rid("order"), 1000, 1000);
        send(&st, refund_event("payment.refunded", pay)).await;
        let ended = !live(&st, &sub).await;
        teardown(&st, &user).await;
        assert!(ended, "the refunded subscription must end");
    }

    #[tokio::test]
    async fn refunding_a_renewal_ends_that_subscription() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let rzp_sub = rid("sub");
        completed_order(&st, &user, &rzp_sub, "pro", 0, vec![json!({ "payment_id": rid("pay"), "signature": "x" })]).await;
        let sub = grant(&st, &user, "Pro", false, Some(&rzp_sub)).await;
        let renewal = rid("pay");
        let charged = json!({
            "event": "subscription.charged",
            "payload": {
                "subscription": { "entity": { "id": rzp_sub, "paid_count": 2 } },
                "payment": { "entity": payment(&renewal, &rid("order"), 1000, 0) },
            }
        });
        assert_eq!(send(&st, charged).await, 200);
        let pay = payment(&renewal, &rid("order"), 1000, 1000);
        send(&st, refund_event("payment.refunded", pay)).await;
        let ended = !live(&st, &sub).await;
        teardown(&st, &user).await;
        assert!(ended, "the refunded renewal must end the subscription");
    }

    #[tokio::test]
    async fn a_lost_dispute_ends_the_plan() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (user, _) = account(&st.pg_pool).await;
        let order = rid("order");
        completed_order(&st, &user, &order, "pro", 0, vec![]).await;
        let sub = grant(&st, &user, "Pro", false, None).await;
        let lost = json!({
            "event": "payment.dispute.lost",
            "payload": {
                "payment": { "entity": payment(&rid("pay"), &order, 1000, 0) },
                "dispute": { "entity": { "id": "disp_1", "amount": 1000, "phase": "chargeback" } },
            }
        });
        send(&st, lost).await;
        let ended = !live(&st, &sub).await;
        teardown(&st, &user).await;
        assert!(ended, "a charged back plan must end");
    }
}
