use axum::{extract::{State, Extension}, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{libs, routes::respond, models};
use reqwest::Client;
use uuid::Uuid;
use sqlx::Row;

#[derive(Deserialize, Debug)]
pub struct CreateOrderPayload {
    pub order_type: String, // "plan" or "quota"
    pub identifier: String, // "plus", "pro", "1tb", "5tb", etc.
    pub currency: String, // "USD", "INR", etc.
    pub gateway: String, // "razorpay", "stripe", etc.
    pub promo_code: Option<String>,
    #[serde(default)]
    pub cycle: Option<String>, // "monthly" (default) or "annual"
}

/// Storage a paid plan grants, in bytes. Free tier is 10 GB (set at signup);
/// Plus adds 200 GB, Pro adds 2 TB.
fn plan_space(identifier: &str) -> i64 {
    match identifier {
        "plus" => 200 * 1024 * 1024 * 1024,      // 200 GB
        "pro" => 2 * 1024_i64.pow(4),            // 2 TB
        _ => 0,
    }
}

/// Display name stored on the subscription row (drives the UI plan label).
fn plan_name(identifier: &str) -> Option<&'static str> {
    match identifier {
        "plus" => Some("Plus"),
        "pro" => Some("Pro"),
        _ => None,
    }
}

/// Postgres INTERVAL literal for a billing cycle. Mapped to fixed literals (never
/// interpolated from user input) so it is safe to inline into the query.
fn cycle_interval(cycle: &str) -> &'static str {
    if cycle == "annual" { "1 year" } else { "1 month" }
}

fn normalize_cycle(cycle: &Option<String>) -> String {
    match cycle.as_deref() {
        Some("annual") => "annual".to_string(),
        _ => "monthly".to_string(),
    }
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(token): Extension<models::UserTokenData>,
    Json(payload): Json<CreateOrderPayload>,
) -> impl IntoResponse {
    // The order is always created for the authenticated user, never a body user_id.
    let user_id = token.id.clone();

    if payload.gateway != "razorpay" {
        return respond(400, "Selected payment gateway is not supported yet", vec![], json!({}));
    }

    // 0.5. Verify User Country Set
    let user_check = sqlx::query!("SELECT country FROM users WHERE id = $1", user_id)
        .fetch_optional(&state.pg_pool)
        .await;

    if let Ok(Some(user)) = user_check {
        if user.country.is_none() || user.country.clone().unwrap_or_default().trim().is_empty() {
             return respond(400, "Please update your profile with your country of residence to continue.", vec![], json!({}));
        }
    } else {
         return respond(500, "Failed to validate user profile", vec![], json!({}));
    }

    let cycle = normalize_cycle(&payload.cycle);

    // 1. Calculate Amount
    let (mut amount, _name) = match calculate_price(&payload.order_type, &payload.identifier, &payload.currency, &cycle) {
        Ok(res) => res,
        Err(_e) => return respond(400, "Invalid plan or identifier", vec![], json!({}))
    };

    // 1.5 Apply Promo Code. Atomically reserve one use: the conditional UPDATE
    // succeeds only when the code is active, unexpired, and under its max-uses
    // cap, and it increments the counter in the same statement — so a code can
    // never be redeemed beyond its limit, even under concurrent requests.
    if let Some(ref code) = payload.promo_code {
        let promo_res = sqlx::query!(
            "UPDATE promo_codes SET uses_count = uses_count + 1 \
             WHERE code = $1 AND active = TRUE \
               AND (expires_at IS NULL OR expires_at > NOW()) \
               AND (max_uses IS NULL OR uses_count < max_uses) \
             RETURNING discount_percentage",
            code
        )
        .fetch_optional(&state.pg_pool)
        .await;

        match promo_res {
            Ok(Some(promo)) => {
                let discount = (amount as f64 * (promo.discount_percentage as f64 / 100.0)) as i64;
                amount -= discount;
                if amount < 0 {
                    amount = 0;
                }
            }
            Ok(None) => {
                return respond(400, "Invalid or expired promo code", vec![], json!({}));
            }
            Err(_e) => {
                return respond(500, "Database error", vec![], json!({}));
            }
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
             user_id,
             if payload.order_type == "plan" { payload.identifier.clone() } else { "".to_string() },
             cycle,
             if payload.order_type == "quota" { calculate_space(&payload.identifier) } else { 0 },
             payload.currency,
             0,
             json!({ "promo_code": payload.promo_code, "applied_discount": 100 })
        )
        .execute(&state.pg_pool)
        .await;

        if let Err(_e) = insert_result {
            return respond(500, "Database Error", vec![], json!({}));
        }

        // Apply Benefits Locally (Duplicated from verify.rs for now)
        let space = if payload.order_type == "quota" { calculate_space(&payload.identifier) } else { 0 };
        if space > 0 {
             let _ = sqlx::query!("UPDATE users SET default_storage_bytes = default_storage_bytes + $1 WHERE id = $2", space, user_id)
                .execute(&state.pg_pool).await;
        } else if let (Some(name), true) = (plan_name(&payload.identifier), payload.order_type == "plan") {
             let interval = cycle_interval(&cycle);
             let sub_res = sqlx::query(
                 &format!("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) \
                           VALUES ($1, $2, $3, NOW() + INTERVAL '{}', FALSE) RETURNING id", interval))
                .bind(name)
                .bind(plan_space(&payload.identifier))
                .bind(&user_id)
                .fetch_optional(&state.pg_pool).await;

             if let Ok(Some(row)) = sub_res {
                let sub_id: String = row.get("id");
                let _ = sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", sub_id, user_id)
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
            return respond(500, "Server configuration error", vec![], json!({}));
        }
    };

    let rzp_order_res = client.post("https://api.razorpay.com/v1/orders")
        .basic_auth(&rzp_config.key_id, Some(&rzp_config.key_secret))
        .json(&json!({
            "amount": amount,
            "currency": payload.currency,
            "payment_capture": 1,
            "receipt": Uuid::new_v4().to_string(),
            "notes": {
                "type": payload.order_type,
                "identifier": payload.identifier,
                "user_id": user_id,
                "promo_code": payload.promo_code.clone().unwrap_or_default()
            }
        }))
        .send()
        .await;

    let rzp_order_json: serde_json::Value = match rzp_order_res {
        Ok(res) => match res.json().await {
             Ok(j) => j,
             Err(_e) => return respond(500, "Failed to parse Gateway response", vec![], json!({}))
        },
        Err(_e) => return respond(500, "Gateway connection failed", vec![], json!({}))
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
         user_id,
         if payload.order_type == "plan" { payload.identifier.clone() } else { "".to_string() },
         cycle,
         add_space,
         payload.currency,
         amount,
         json!({ "gateway_res": rzp_order_json, "promo_code": payload.promo_code })
    )
    .execute(&state.pg_pool)
    .await;

    if let Err(e) = insert_result {
        println!("DB Error: {:?}", e);
        return respond(500, "Database Error", vec![], json!({}));
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

// Helper for pricing. Annual = ~10 months (two months free).
fn calculate_price(order_type: &str, identifier: &str, currency: &str, cycle: &str) -> anyhow::Result<(i64, String)> {
    let multiplier = 100; // Cents or Paise
    let annual = cycle == "annual";

    let whole = match (currency, order_type, identifier) {
        // ---- USD ----
        ("USD", "plan", "plus") => if annual { 39 } else { 4 },
        ("USD", "plan", "pro") => if annual { 96 } else { 10 },
        ("USD", "quota", "1tb") => 5,
        ("USD", "quota", "5tb") => 20,
        ("USD", "quota", "10tb") => 35,
        ("USD", "quota", "20tb") => 60,

        // ---- EUR ----
        ("EUR", "plan", "plus") => if annual { 39 } else { 4 },
        ("EUR", "plan", "pro") => if annual { 90 } else { 9 },
        ("EUR", "quota", "1tb") => 4,
        ("EUR", "quota", "5tb") => 18,
        ("EUR", "quota", "10tb") => 32,
        ("EUR", "quota", "20tb") => 55,

        // ---- INR ----
        ("INR", "plan", "plus") => if annual { 3490 } else { 349 },
        ("INR", "plan", "pro") => if annual { 8990 } else { 899 },
        ("INR", "quota", "1tb") => 399,
        ("INR", "quota", "5tb") => 1599,
        ("INR", "quota", "10tb") => 2999,
        ("INR", "quota", "20tb") => 4999,

        _ => return Err(anyhow::anyhow!("Unknown item or currency"))
    };
    let amount = whole * multiplier;

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
