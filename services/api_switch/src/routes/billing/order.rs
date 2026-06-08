use axum::{extract::{State, Extension}, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{libs, routes::respond, models};
use reqwest::Client;
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub struct CreateOrderPayload {
    pub order_type: String, // "plan" or "quota"
    pub identifier: String, // "pro", "1tb", "5tb", etc.
    pub currency: String, // "USD", "INR", etc.
    pub user_id: String,
    pub gateway: String, // "razorpay", "stripe", etc.
    pub promo_code: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<CreateOrderPayload>,
) -> impl IntoResponse {

    if payload.gateway != "razorpay" {
        return respond(400, "Selected payment gateway is not supported yet", vec![], json!({}));
    }

    // 0.5. Verify User Country Set
    let user_check = sqlx::query!("SELECT country FROM users WHERE id = $1", payload.user_id)
        .fetch_optional(&state.pg_pool)
        .await;

    if let Ok(Some(user)) = user_check {
        if user.country.is_none() || user.country.clone().unwrap_or_default().trim().is_empty() {
             return respond(400, "Please update your profile with your country of residence to continue.", vec![], json!({}));
        }
    } else {
         return respond(500, "Failed to validate user profile", vec![], json!({}));
    }

    // 1. Calculate Amount
    let (mut amount, _name) = match calculate_price(&payload.order_type, &payload.identifier, &payload.currency) {
        Ok(res) => res,
        Err(e) => return respond(400, "Invalid plan or identifier", vec![e.to_string()], json!({}))
    };

    // 1.5 Apply Promo Code
    let mut _applied_promo: Option<models::PromoCode> = None;
    if let Some(ref code) = payload.promo_code {
        let promo_res = sqlx::query_as!(
            models::PromoCode,
            "SELECT code, discount_percentage, duration, COALESCE(active, TRUE) as \"active!\" FROM promo_codes WHERE code = $1 AND active = TRUE",
            code
        )
        .fetch_optional(&state.pg_pool)
        .await;

        if let Ok(Some(promo)) = promo_res {
            let discount = (amount as f64 * (promo.discount_percentage as f64 / 100.0)) as i64;
            amount -= discount;
            if amount < 0 {
                amount = 0;
            }
            _applied_promo = Some(promo);
        } else {
             // Optional: Return error if code invalid? Or just ignore? 
             // Better to return 400 so UI can show "Invalid code"
             return respond(400, "Invalid or expired promo code", vec![], json!({}));
        }
    }

    // 1.8 Handle 100% Discount (Amount <= 0)
    if amount <= 0 {
         let local_order_id = format!("free_{}", Uuid::new_v4());
         
         // Insert Completed Order
         let insert_result = sqlx::query!(
            "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, payment_gateway, currency, amount, status, details)
             VALUES ($1, $2, $3, $4, $5, 'free', $6, $7, 'completed', $8)",
             local_order_id,
             payload.user_id,
             if payload.order_type == "plan" { payload.identifier.clone() } else { "".to_string() },
             "monthly", 
             if payload.order_type == "quota" { calculate_space(&payload.identifier) } else { 0 },
             payload.currency,
             0,
             json!({ "promo_code": payload.promo_code, "applied_discount": 100 })
        )
        .execute(&state.pg_pool)
        .await;

        if let Err(e) = insert_result {
            return respond(500, "Database Error", vec![e.to_string()], json!({}));
        }

        // Apply Benefits Locally (Duplicated from verify.rs for now)
        let space = if payload.order_type == "quota" { calculate_space(&payload.identifier) } else { 0 };
        if space > 0 {
             let _ = sqlx::query!("UPDATE users SET default_storage_bytes = default_storage_bytes + $1 WHERE id = $2", space, payload.user_id)
                .execute(&state.pg_pool).await;
        } else if payload.identifier == "pro" {
             let sub_res = sqlx::query!("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) VALUES ('Pro', 1099511627776, $1, NOW() + INTERVAL '1 month', FALSE) RETURNING id", payload.user_id)
                .fetch_optional(&state.pg_pool).await;
             
             if let Ok(Some(row)) = sub_res {
                let _ = sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", row.id, payload.user_id)
                    .execute(&state.pg_pool).await;
             }
        }

        return respond(200, "Order completed (Free)", vec![], json!({
             "success": true,
             "amount": 0,
             "currency": payload.currency
        }));
    }

    // 2. Create Order on Razorpay
    let client = Client::new();
    let rzp_config = match libs::configs::razorpay_config() {
        Ok(c) => c,
        Err(e) => {
            println!("[BILLING_ORDER] Config Error: {:?}", e);
            return respond(500, "Server configuration error", vec![e.to_string()], json!({}));
        }
    };

    let rzp_order_res = client.post("https://api.razorpay.com/v1/orders")
        .basic_auth(&rzp_config.key_id, Some(&rzp_config.key_secret))
        .json(&json!({
            "amount": amount,
            "currency": payload.currency,
            "receipt": Uuid::new_v4().to_string(),
            "notes": {
                "type": payload.order_type,
                "identifier": payload.identifier,
                "user_id": payload.user_id,
                "promo_code": payload.promo_code.clone().unwrap_or_default()
            }
        }))
        .send()
        .await;

    let rzp_order_json: serde_json::Value = match rzp_order_res {
        Ok(res) => match res.json().await {
             Ok(j) => j,
             Err(e) => return respond(500, "Failed to parse Gateway response", vec![e.to_string()], json!({}))
        },
        Err(e) => return respond(500, "Gateway connection failed", vec![e.to_string()], json!({}))
    };

    if rzp_order_json.get("error").is_some() {
         return respond(500, "Payment Gateway Error", vec![rzp_order_json["error"]["description"].as_str().unwrap_or("Unknown").to_string()], json!({}));
    }

    let rzp_order_id = match rzp_order_json.get("id") {
        Some(id) => id.as_str().unwrap_or("").to_string(),
        None => return respond(500, "Invalid Gateway response", vec![], json!({}))
    };

    // 3. Save Order to Database
    let mut add_space: i64 = 0;
    if payload.order_type == "quota" {
        add_space = match payload.identifier.as_str() {
            "1tb" => 1000 * 1024 * 1024 * 1024,
            "5tb" => 5000 * 1024 * 1024 * 1024,
            "10tb" => 10000 * 1024 * 1024 * 1024,
            "20tb" => 20000 * 1024 * 1024 * 1024,
            _ => 0
        };
    }

    let insert_result = sqlx::query!(
        "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, payment_gateway, currency, amount, status, details)
         VALUES ($1, $2, $3, $4, $5, 'razorpay', $6, $7, 'pending', $8)",
         rzp_order_id,
         payload.user_id,
         if payload.order_type == "plan" { payload.identifier.clone() } else { "".to_string() },
         "monthly", // Hardcoded for now
         add_space,
         payload.currency,
         amount,
         json!({ "gateway_res": rzp_order_json, "promo_code": payload.promo_code })
    )
    .execute(&state.pg_pool)
    .await;

    if let Err(e) = insert_result {
        println!("DB Error: {:?}", e);
        return respond(500, "Database Error", vec![e.to_string()], json!({}));
    }

    respond(200, "Order created", vec![], json!({
         "order_id": rzp_order_id,
         "amount": amount,
         "currency": payload.currency,
         "key_id": rzp_config.key_id
    }))
}

/// Real prices only in production. In dev/staging we charge the smallest valid
/// amount so live payment flows can be exercised end to end without spending real
/// money. Driven by APP_ENV; defaults to prod when unset so a misconfigured prod
/// box never silently under-charges.
fn is_production() -> bool {
    std::env::var("APP_ENV")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            v == "prod" || v == "production"
        })
        .unwrap_or(true)
}

// Helper for pricing
fn calculate_price(order_type: &str, identifier: &str, currency: &str) -> anyhow::Result<(i64, String)> {
    let multiplier = 100; // Cents or Paise

    let amount = match (currency, order_type, identifier) {
        // USD Pricing
        ("USD", "plan", "pro") => 9 * multiplier,
        ("USD", "quota", "1tb") => 5 * multiplier,
        ("USD", "quota", "5tb") => 20 * multiplier,
        ("USD", "quota", "10tb") => 35 * multiplier,
        ("USD", "quota", "20tb") => 60 * multiplier,

        // EUR Pricing
        ("EUR", "plan", "pro") => 8 * multiplier,
        ("EUR", "quota", "1tb") => 4 * multiplier, // Approx equivalent
        ("EUR", "quota", "5tb") => 18 * multiplier,
        ("EUR", "quota", "10tb") => 32 * multiplier,
        ("EUR", "quota", "20tb") => 55 * multiplier,

         // INR Pricing
        ("INR", "plan", "pro") => 950 * multiplier,
        ("INR", "quota", "1tb") => 399 * multiplier,
        ("INR", "quota", "5tb") => 1599 * multiplier,
        ("INR", "quota", "10tb") => 2999 * multiplier,
        ("INR", "quota", "20tb") => 4999 * multiplier,

        _ => return Err(anyhow::anyhow!("Unknown item or currency"))
    };

    // Floor the charge to a near-zero amount outside production. 100 (= 1 whole
    // unit: ₹1 / $1 / €1) clears Razorpay's per-currency minimum order amount.
    if !is_production() {
        return Ok((100, format!("{} {} (test)", identifier, order_type)));
    }

    Ok((amount, format!("{} {}", identifier, order_type)))
}

fn calculate_space(identifier: &str) -> i64 {
    match identifier {
        "1tb" => 1000 * 1024 * 1024 * 1024,
        "5tb" => 5000 * 1024 * 1024 * 1024,
        "10tb" => 10000 * 1024 * 1024 * 1024,
        "20tb" => 20000 * 1024 * 1024 * 1024,
        _ => 0
    }
}
