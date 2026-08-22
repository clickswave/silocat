use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, routes::respond};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use reqwest::Client;
use sqlx::Row;

/// Paid plan → (display name, storage bytes). Canonical source: libs::plans.
fn plan_grant(identifier: &str) -> Option<(&'static str, i64)> {
    crate::libs::plans::plan_grant(identifier)
}

#[derive(Deserialize, Debug)]
pub struct VerifyPayload {
    // A one-time order sends order_id; a subscription checkout sends
    // subscription_id instead (Razorpay Checkout returns razorpay_subscription_id).
    #[serde(default)]
    pub order_id: Option<String>,
    #[serde(default)]
    pub subscription_id: Option<String>,
    pub payment_id: String,
    pub signature: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<VerifyPayload>,
) -> impl IntoResponse {
    // Orders and subscriptions share this endpoint. A subscription checkout
    // returns razorpay_subscription_id (and signs payment_id|subscription_id); a
    // one-time order returns order_id (and signs order_id|payment_id). Either way
    // the pending row is keyed by reference_id (see order.rs).
    let is_subscription = payload.subscription_id.is_some();
    let reference = match payload.subscription_id.clone().or_else(|| payload.order_id.clone()) {
        Some(r) => r,
        None => return respond(400, "Missing order or subscription id", vec![], json!({})),
    };

    // 1. Fetch the order.
    let order = match sqlx::query!("SELECT * FROM orders WHERE reference_id = $1", reference)
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(o)) => o,
        Ok(None) => return respond(404, "Order not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database Error", vec![], json!({})),
    };

    if order.status == "completed" {
        return respond(200, "Payment Verified (Already Processed)", vec![], json!({ "success": true }));
    }

    if order.payment_gateway != "razorpay" {
        return respond(400, "Unsupported Gateway for Verification", vec![], json!({}));
    }

    let rzp_config = match libs::configs::razorpay_config() {
        Ok(c) => c,
        Err(_e) => return respond(500, "Configuration Error", vec![], json!({})),
    };

    // 2. Verify the checkout handshake signature (constant-time). Razorpay signs
    //    payment_id|subscription_id for a subscription and order_id|payment_id for
    //    a one-time order.
    let msg = if is_subscription {
        format!("{}|{}", payload.payment_id, reference)
    } else {
        format!("{}|{}", reference, payload.payment_id)
    };
    let mut mac = Hmac::<Sha256>::new_from_slice(rzp_config.key_secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(msg.as_bytes());
    let provided = match hex::decode(payload.signature.trim()) {
        Ok(b) => b,
        Err(_) => return respond(400, "Invalid Signature", vec!["Payment verification failed".to_string()], json!({})),
    };
    if mac.verify_slice(&provided).is_err() {
        return respond(400, "Invalid Signature", vec!["Payment verification failed".to_string()], json!({}));
    }

    // 3. A valid handshake signature does not prove the money was captured, so
    //    confirm server-to-server that the payment is captured for the exact
    //    amount and currency of this order.
    let client = Client::new();
    let pay: serde_json::Value = match client
        .get(format!("https://api.razorpay.com/v1/payments/{}", payload.payment_id))
        .basic_auth(&rzp_config.key_id, Some(&rzp_config.key_secret))
        .send()
        .await
    {
        Ok(r) => match r.json().await {
            Ok(j) => j,
            Err(_e) => return respond(502, "Gateway response error", vec![], json!({})),
        },
        Err(_e) => return respond(502, "Gateway connection failed", vec![], json!({})),
    };

    let captured = pay.get("status").and_then(|s| s.as_str()) == Some("captured");
    let amount_ok = pay.get("amount").and_then(|a| a.as_i64()) == Some(order.amount);
    let currency_ok = pay.get("currency").and_then(|c| c.as_str()) == Some(order.currency.as_str());
    if !(captured && amount_ok && currency_ok) {
        return respond(
            400,
            "Payment not captured",
            vec!["The payment was not captured for the expected amount.".to_string()],
            json!({}),
        );
    }

    // 4. Claim the order and apply benefits atomically. The conditional UPDATE
    //    (status = 'pending') serialises concurrent verifies: only the first
    //    claims it, so benefits are never granted twice: and the transaction
    //    means any failed grant rolls the claim back (status stays 'pending',
    //    retryable) instead of leaving a paid-but-unfulfilled order.
    let mut tx = match state.pg_pool.begin().await {
        Ok(t) => t,
        Err(_e) => return respond(500, "Database Error", vec![], json!({})),
    };

    let claimed = sqlx::query!(
        "UPDATE orders SET status = 'completed', transactions = array_append(transactions, $1) \
         WHERE reference_id = $2 AND status = 'pending' RETURNING reference_id",
        json!({ "payment_id": payload.payment_id, "signature": payload.signature }),
        reference
    )
    .fetch_optional(&mut *tx)
    .await;

    match claimed {
        Ok(Some(_)) => {} // we claimed it first
        Ok(None) => {
            // Another concurrent verify already completed it.
            let _ = tx.rollback().await;
            return respond(200, "Payment Verified (Already Processed)", vec![], json!({ "success": true }));
        }
        Err(_e) => {
            let _ = tx.rollback().await;
            return respond(500, "Failed to update order", vec![], json!({}));
        }
    }

    if order.additional_space > 0 {
        if let Err(_e) = sqlx::query!(
            "UPDATE users SET default_storage_bytes = default_storage_bytes + $1 WHERE id = $2",
            order.additional_space,
            order.user_id
        )
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return respond(500, "Failed to update quota", vec![], json!({}));
        }
    } else if let Some((plan_display, plan_space)) = plan_grant(&order.subscription_name) {
        // Interval literal is chosen from a fixed set (never user input), safe to inline.
        let interval = if order.subscription_cycle == "annual" { "1 year" } else { "1 month" };
        // Replace any currently-active PAID plan (invited = false) before granting
        // the new one, so a renewal or upgrade bought before the old plan lapses
        // does not STACK quota (two active Pro rows would sum to 4 TB). Promo and
        // invite grants (invited = true) are left untouched.
        if let Err(_e) = sqlx::query(
            "UPDATE subscriptions SET expires_on = NOW() \
             WHERE created_by = $1 AND invited = false AND expires_on > NOW()"
        )
        .bind(&order.user_id)
        .execute(&mut *tx)
        .await
        {
            let _ = tx.rollback().await;
            return respond(500, "Failed to update subscription", vec![], json!({}));
        }
        // Link the local subscription to the Razorpay Subscription (when this
        // grant came from one) so subscription.charged webhooks can extend it.
        let sub_res = sqlx::query(
            &format!(
                "INSERT INTO subscriptions (name, additional_space, created_by, expires_on, razorpay_subscription_id) \
                 VALUES ($1, $2, $3, NOW() + INTERVAL '{}', $4) RETURNING id",
                interval
            )
        )
        .bind(plan_display)
        .bind(plan_space)
        .bind(&order.user_id)
        .bind(if is_subscription { Some(reference.clone()) } else { None::<String> })
        .fetch_one(&mut *tx)
        .await;

        match sub_res {
            Ok(row) => {
                let sub_id: String = row.get("id");
                if let Err(_e) = sqlx::query!(
                    "UPDATE users SET subscription_id = $1 WHERE id = $2",
                    sub_id,
                    order.user_id
                )
                .execute(&mut *tx)
                .await
                {
                    let _ = tx.rollback().await;
                    return respond(500, "Failed to link subscription", vec![], json!({}));
                }
            }
            Err(_e) => {
                let _ = tx.rollback().await;
                return respond(500, "Failed to create subscription", vec![], json!({}));
            }
        }
    }

    if let Err(_e) = tx.commit().await {
        return respond(500, "Failed to finalize order", vec![], json!({}));
    }

    respond(200, "Payment Verified and Benefits Applied", vec![], json!({ "success": true }))
}
