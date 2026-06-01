use axum::{extract::{State, Extension}, Json, response::IntoResponse};
use serde::{Deserialize};
use serde_json::json;
use crate::{libs, routes::respond, models};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

#[derive(Deserialize, Debug)]
pub struct VerifyPayload {
    pub order_id: String,
    pub payment_id: String,
    pub signature: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<VerifyPayload>,
) -> impl IntoResponse {

    // 1. Fetch Order
    let order_opt = sqlx::query!(
        "SELECT * FROM orders WHERE reference_id = $1",
        payload.order_id
    )
    .fetch_optional(&state.pg_pool)
    .await;

    let order = match order_opt {
        Ok(Some(o)) => o,
        Ok(None) => return respond(404, "Order not found", vec![], json!({})),
        Err(e) => return respond(500, "Database Error", vec![e.to_string()], json!({}))
    };

    if order.status == "completed" {
         return respond(200, "Payment Verified (Already Processed)", vec![], json!({"success": true}));
    }

    // 2. Verify Signature (Based on Gateway)
    if order.payment_gateway == "razorpay" {
        let rzp_config = match libs::configs::razorpay_config() {
            Ok(c) => c,
            Err(e) => return respond(500, "Configuration Error", vec![e.to_string()], json!({}))
        };

        let msg = format!("{}|{}", payload.order_id, payload.payment_id);
        let mut mac = Hmac::<Sha256>::new_from_slice(rzp_config.key_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(msg.as_bytes());
        let expected_signature = hex::encode(mac.finalize().into_bytes());

        if expected_signature != payload.signature {
            return respond(400, "Invalid Signature", vec!["Payment verification failed".to_string()], json!({}));
        }
    } else {
        return respond(400, "Unsupported Gateway for Verification", vec![], json!({}));
    }

    // 3. Mark Order as Completed
    if let Err(e) = sqlx::query!(
        "UPDATE orders SET status = 'completed', transactions = array_append(transactions, $1) WHERE reference_id = $2",
        json!({ "payment_id": payload.payment_id, "signature": payload.signature }),
        payload.order_id
    )
    .execute(&state.pg_pool)
    .await
    {
        return respond(500, "Failed to update order", vec![e.to_string()], json!({}));
    }

    // 4. Apply Benefits (Plan or Quota)
    if order.additional_space > 0 {
        // Add User Storage
         if let Err(e) = sqlx::query!(
            "UPDATE users SET default_storage_bytes = default_storage_bytes + $1 WHERE id = $2",
            order.additional_space,
            order.user_id
        )
        .execute(&state.pg_pool)
        .await
        {
             println!("CRITICAL: Payment secured but storage update failed for order {}", payload.order_id);
             return respond(500, "Failed to update quota", vec![e.to_string()], json!({}));
        }
    } else if order.subscription_name == "pro" {
        // Create Subscription
        let sub_res = sqlx::query!(
            "INSERT INTO subscriptions (name, additional_space, created_by, expires_on) 
             VALUES ('Pro', 1099511627776, $1, NOW() + INTERVAL '1 month')
             RETURNING id",
             order.user_id
        )
        .fetch_one(&state.pg_pool)
        .await;

        match sub_res {
            Ok(row) => {
                 if let Err(e) = sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", row.id, order.user_id)
                     .execute(&state.pg_pool)
                     .await 
                 {
                      return respond(500, "Failed to link subscription", vec![e.to_string()], json!({}));
                 }
            },
            Err(e) => return respond(500, "Failed to create subscription", vec![e.to_string()], json!({}))
        }
    }

    respond(200, "Payment Verified and Benefits Applied", vec![], json!({"success": true}))
}
