use axum::{extract::{State, Extension}, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{libs, routes::respond, models};
use reqwest::Client;
use uuid::Uuid;
use sqlx::Row;

#[derive(Deserialize, Debug)]
pub struct CreateOrderPayload {
    pub order_type: String, // "plan" (the only thing sold)
    pub identifier: String, // "plus" or "pro"
    pub currency: String, // "USD", "INR", etc.
    pub gateway: String, // "razorpay", "stripe", etc.
    pub promo_code: Option<String>,
    #[serde(default)]
    pub cycle: Option<String>, // "monthly" (default) or "annual"
}

// Plan grants + cycle intervals live in one place now (libs::plans); these thin
// wrappers keep the local call sites unchanged.
use crate::libs::plans;

fn plan_space(identifier: &str) -> i64 { plans::plan_space(identifier) }

fn plan_name(identifier: &str) -> Option<&'static str> { plans::plan_name(identifier) }

fn cycle_interval(cycle: &str) -> &'static str { plans::cycle_interval(cycle) }

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

    // 1.5 Apply Promo Code. This only reads the discount. The use is taken in
    // the same transaction that writes the order row (record_order), so a use
    // is spent only by an order that exists, and the abandoned-order sweep
    // can hand it back.
    let promo = match payload.promo_code.as_deref() {
        Some(code) => match sqlx::query_scalar::<_, i32>(&format!(
            "SELECT discount_percentage FROM promo_codes WHERE {}",
            PROMO_REDEEMABLE
        ))
        .bind(code)
        .bind(&user_id)
        .fetch_optional(&state.pg_pool)
        .await
        {
            Ok(Some(pct)) => Some((code, pct)),
            Ok(None) => return respond(400, "Invalid or expired promo code", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        },
        None => None,
    };
    if let Some((_, pct)) = promo {
        let discount = (amount as f64 * (pct as f64 / 100.0)) as i64;
        amount -= discount;
        if amount < 0 {
            amount = 0;
        }
    }

    // 1.8 Handle 100% Discount (Amount <= 0)
    if amount <= 0 {
         let local_order_id = format!("free_{}", Uuid::new_v4());

         let mut tx = match record_order(&state, &user_id, promo).await {
             Ok(Some(tx)) => tx,
             Ok(None) => return respond(400, "Invalid or expired promo code", vec![], json!({})),
             Err(_e) => return respond(500, "Database Error", vec![], json!({})),
         };

         // Insert Completed Order
         let insert_result = sqlx::query!(
            "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, payment_gateway, currency, amount, status, details)
             VALUES ($1, $2, $3, $4, $5, 'free', $6, $7, 'completed', $8)",
             local_order_id,
             user_id,
             payload.identifier.clone(),
             cycle,
             0_i64,
             payload.currency,
             0,
             json!({ "promo_code": payload.promo_code, "applied_discount": 100 })
        )
        .execute(&mut *tx)
        .await;

        if let Err(_e) = insert_result {
            return respond(500, "Database Error", vec![], json!({}));
        }

        // Apply Benefits Locally (Duplicated from verify.rs for now)
        if let Some(name) = plan_name(&payload.identifier) {
             let interval = cycle_interval(&cycle);
             // Replace any active paid plan first so the grant doesn't stack quota.
             let replaced = sqlx::query(
                 "UPDATE subscriptions SET expires_on = NOW() \
                  WHERE created_by = $1 AND invited = false AND expires_on > NOW()")
                .bind(&user_id)
                .execute(&mut *tx).await;
             let sub_res = sqlx::query(
                 &format!("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) \
                           VALUES ($1, $2, $3, NOW() + INTERVAL '{}', FALSE) RETURNING id", interval))
                .bind(name)
                .bind(plan_space(&payload.identifier))
                .bind(&user_id)
                .fetch_one(&mut *tx).await;
             let linked = match (replaced, sub_res) {
                 (Ok(_), Ok(row)) => {
                     let sub_id: String = row.get("id");
                     sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", sub_id, user_id)
                         .execute(&mut *tx).await
                         .is_ok()
                 }
                 _ => false,
             };
             if !linked {
                 return respond(500, "Database Error", vec![], json!({}));
             }
        }

        if let Err(_e) = tx.commit().await {
            return respond(500, "Database Error", vec![], json!({}));
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

    // 2a. Recurring plans. If a Razorpay Plan is configured for this
    // (plan, cycle, currency), create an auto-renewing SUBSCRIPTION (a real
    // mandate) instead of a one-time order, which is what turns paid plans into
    // actual recurring revenue. When no plan id is configured the code falls
    // through to the one-time order below, so behaviour is unchanged until the
    // plans are created and their ids wired in via env
    // (scripts/silocat_razorpay_plans.py).
    if payload.order_type == "plan" {
        if let Some(plan_id) =
            crate::libs::plans::razorpay_plan_id(&payload.identifier, &cycle, &payload.currency)
        {
            let sub_res = client
                .post("https://api.razorpay.com/v1/subscriptions")
                .basic_auth(&rzp_config.key_id, Some(&rzp_config.key_secret))
                .json(&json!({
                    "plan_id": plan_id,
                    "total_count": crate::libs::plans::SUBSCRIPTION_TOTAL_COUNT,
                    "customer_notify": 1,
                    "notes": {
                        "type": "plan",
                        "identifier": payload.identifier,
                        "user_id": user_id,
                        "cycle": cycle,
                    }
                }))
                .send()
                .await;

            let sub_json: serde_json::Value = match sub_res {
                Ok(r) => match r.json().await {
                    Ok(j) => j,
                    Err(_e) => return respond(500, "Failed to parse Gateway response", vec![], json!({})),
                },
                Err(_e) => return respond(500, "Gateway connection failed", vec![], json!({})),
            };
            if sub_json.get("error").is_some() {
                return respond(500, "Payment Gateway Error",
                    vec![sub_json["error"]["description"].as_str().unwrap_or("Unknown").to_string()],
                    json!({}));
            }
            let sub_id = match sub_json.get("id").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => return respond(500, "Invalid Gateway response", vec![], json!({})),
            };

            // Track the pending activation as an order row keyed by the
            // subscription id, so /billing/verify and the webhook find it the same
            // way they find a one-time order.
            let mut tx = match record_order(&state, &user_id, promo).await {
                Ok(Some(tx)) => tx,
                Ok(None) => return respond(400, "Invalid or expired promo code", vec![], json!({})),
                Err(_e) => return respond(500, "Database Error", vec![], json!({})),
            };
            let ins = sqlx::query!(
                "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, payment_gateway, currency, amount, status, details) \
                 VALUES ($1, $2, $3, $4, 0, 'razorpay', $5, $6, 'pending', $7)",
                sub_id,
                user_id,
                payload.identifier,
                cycle,
                payload.currency,
                amount,
                json!({ "gateway_res": sub_json, "kind": "subscription", "promo_code": payload.promo_code })
            )
            .execute(&mut *tx)
            .await;
            if let Err(e) = ins {
                println!("DB Error: {:?}", e);
                return respond(500, "Database Error", vec![], json!({}));
            }
            if let Err(_e) = tx.commit().await {
                return respond(500, "Database Error", vec![], json!({}));
            }

            return respond(200, "Subscription created", vec![], json!({
                "subscription_id": sub_id,
                "amount": amount,
                "currency": payload.currency,
                "key_id": rzp_config.key_id,
                "recurring": true
            }));
        }
    }

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
    let mut tx = match record_order(&state, &user_id, promo).await {
        Ok(Some(tx)) => tx,
        Ok(None) => return respond(400, "Invalid or expired promo code", vec![], json!({})),
        Err(_e) => return respond(500, "Database Error", vec![], json!({})),
    };
    let insert_result = sqlx::query!(
        "INSERT INTO orders (reference_id, user_id, subscription_name, subscription_cycle, additional_space, payment_gateway, currency, amount, status, details)
         VALUES ($1, $2, $3, $4, $5, 'razorpay', $6, $7, 'pending', $8)",
         rzp_order_id,
         user_id,
         payload.identifier.clone(),
         cycle,
         0_i64,
         payload.currency,
         amount,
         json!({ "gateway_res": rzp_order_json, "promo_code": payload.promo_code })
    )
    .execute(&mut *tx)
    .await;

    if let Err(e) = insert_result {
        println!("DB Error: {:?}", e);
        return respond(500, "Database Error", vec![], json!({}));
    }
    if let Err(_e) = tx.commit().await {
        return respond(500, "Database Error", vec![], json!({}));
    }

    respond(200, "Order created", vec![], json!({
         "order_id": rzp_order_id,
         "amount": amount,
         "currency": payload.currency,
         "key_id": rzp_config.key_id
    }))
}

/// When code $1 can still be redeemed by account $2: active, unexpired, under
/// its cap, and not already used by an order of that account that was paid,
/// given away free, or refunded. A checkout that was never paid does not count,
/// so someone who closed the payment sheet can try again.
const PROMO_REDEEMABLE: &str = "code = $1 AND active = TRUE \
     AND (expires_at IS NULL OR expires_at > NOW()) \
     AND (max_uses IS NULL OR uses_count < max_uses) \
     AND NOT EXISTS (SELECT 1 FROM orders o WHERE o.user_id = $2 \
                     AND o.details->>'promo_code' = promo_codes.code \
                     AND (order_is_settled(o.status) OR o.status = 'refunded'))";

/// Open the transaction an order row is written in. It locks the account's
/// row, so one account's orders are written one at a time, and when the order
/// uses a promo code it takes one use of it, at the same discount the price
/// was worked out with. `None` means the code can no longer be redeemed: the
/// cap was reached, or this account redeemed it in the meantime.
async fn record_order(
    state: &crate::AppState,
    user_id: &str,
    promo: Option<(&str, i32)>,
) -> Result<Option<sqlx::Transaction<'static, sqlx::Postgres>>, sqlx::Error> {
    let mut tx = state.pg_pool.begin().await?;
    sqlx::query("SELECT 1 FROM users WHERE id = $1 FOR UPDATE")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;
    if let Some((code, pct)) = promo {
        let taken = sqlx::query(&format!(
            "UPDATE promo_codes SET uses_count = uses_count + 1 \
             WHERE {} AND discount_percentage = $3 RETURNING code",
            PROMO_REDEEMABLE
        ))
        .bind(code)
        .bind(user_id)
        .bind(pct)
        .fetch_optional(&mut *tx)
        .await?;
        if taken.is_none() {
            tx.rollback().await?;
            return Ok(None);
        }
    }
    Ok(Some(tx))
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

        // ---- EUR ----
        ("EUR", "plan", "plus") => if annual { 39 } else { 4 },
        ("EUR", "plan", "pro") => if annual { 90 } else { 9 },

        // ---- INR ----
        ("INR", "plan", "plus") => if annual { 3490 } else { 349 },
        ("INR", "plan", "pro") => if annual { 8990 } else { 899 },

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

#[cfg(test)]
mod tests {
    //! These place orders for accounts in a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Only the free path
    //! (a 100% code) and refusals are driven, so no gateway is called.

    use super::*;
    use crate::test_support::{account, forget, state, SETTLING_ORDERS};

    fn token(id: &str) -> models::UserTokenData {
        models::UserTokenData {
            id: id.to_string(),
            email: String::new(),
            profile_image: None,
            email_verified: true,
            password_set: true,
            username: String::new(),
            api_key: String::new(),
            account_type: "personal".to_string(),
            subscription: None,
            default_storage_bytes: 0,
            country: Some("IN".to_string()),
            bio: None,
        }
    }

    fn body(order_type: &str, identifier: &str, code: &str) -> CreateOrderPayload {
        CreateOrderPayload {
            order_type: order_type.to_string(),
            identifier: identifier.to_string(),
            currency: "USD".to_string(),
            gateway: "razorpay".to_string(),
            promo_code: Some(code.to_string()),
            cycle: None,
        }
    }

    async fn place(st: &crate::AppState, id: &str, payload: CreateOrderPayload) -> u16 {
        handle(State(st.clone()), Extension(token(id)), Json(payload)).await.into_response().status().as_u16()
    }

    /// A signed-in account with a country, and a 100% code with no cap.
    async fn setup(st: &crate::AppState) -> (String, String) {
        let (id, _) = account(&st.pg_pool).await;
        sqlx::query("UPDATE users SET country = 'IN' WHERE id = $1").bind(&id).execute(&st.pg_pool).await.unwrap();
        let code = format!("FREE{}", crate::libs::rng::string(10)).to_uppercase();
        sqlx::query("INSERT INTO promo_codes (code, discount_percentage, duration, active) VALUES ($1, 100, '1 month', TRUE)")
            .bind(&code)
            .execute(&st.pg_pool)
            .await
            .unwrap();
        (id, code)
    }

    async fn teardown(st: &crate::AppState, id: &str, code: &str) {
        sqlx::query("DELETE FROM orders WHERE user_id = $1").bind(id).execute(&st.pg_pool).await.unwrap();
        sqlx::query("DELETE FROM promo_codes WHERE code = $1").bind(code).execute(&st.pg_pool).await.unwrap();
        forget(&st.pg_pool, id).await;
    }

    async fn storage(st: &crate::AppState, id: &str) -> i64 {
        sqlx::query_scalar("SELECT default_storage_bytes FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    async fn completed_orders(st: &crate::AppState, id: &str) -> i64 {
        sqlx::query_scalar("SELECT COUNT(*) FROM orders WHERE user_id = $1 AND status = 'completed'")
            .bind(id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    #[test]
    fn storage_is_not_sold_outside_a_plan() {
        for currency in ["USD", "EUR", "INR"] {
            for size in ["1tb", "5tb", "10tb", "20tb"] {
                assert!(calculate_price("quota", size, currency, "monthly").is_err(), "{currency} {size}");
            }
            assert!(calculate_price("plan", "pro", currency, "monthly").is_ok());
        }
    }

    #[tokio::test]
    async fn a_free_code_buys_no_permanent_storage() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (id, code) = setup(&st).await;
        let before = storage(&st, &id).await;
        let first = place(&st, &id, body("quota", "20tb", &code)).await;
        let second = place(&st, &id, body("quota", "20tb", &code)).await;
        let after = storage(&st, &id).await;
        teardown(&st, &id, &code).await;
        assert_eq!((first, second), (400, 400));
        assert_eq!(after, before, "a quota order must not add storage");
    }

    #[tokio::test]
    async fn a_free_code_gives_one_plan_per_account() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (id, code) = setup(&st).await;
        let first = place(&st, &id, body("plan", "pro", &code)).await;
        let second = place(&st, &id, body("plan", "pro", &code)).await;
        let completed = completed_orders(&st, &id).await;
        teardown(&st, &id, &code).await;
        assert_eq!(first, 200);
        assert_eq!(second, 400, "the same account must not redeem a code twice");
        assert_eq!(completed, 1);
    }

    #[tokio::test]
    async fn parallel_orders_redeem_a_free_code_once() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (id, code) = setup(&st).await;
        let mut tasks = Vec::new();
        for _ in 0..6 {
            let (st, id, code) = (st.clone(), id.clone(), code.clone());
            tasks.push(tokio::spawn(async move { place(&st, &id, body("plan", "pro", &code)).await }));
        }
        let mut ok = 0;
        for t in tasks {
            if t.await.unwrap() == 200 {
                ok += 1;
            }
        }
        let completed = completed_orders(&st, &id).await;
        let live: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM subscriptions WHERE created_by = $1 AND expires_on > NOW()")
            .bind(&id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        teardown(&st, &id, &code).await;
        assert_eq!(ok, 1);
        assert_eq!(completed, 1);
        assert_eq!(live, 1);
    }

    #[tokio::test]
    async fn a_refused_order_does_not_spend_a_capped_code() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let _settling = SETTLING_ORDERS.lock().await;
        let (id, code) = setup(&st).await;
        sqlx::query("UPDATE promo_codes SET max_uses = 5 WHERE code = $1").bind(&code).execute(&st.pg_pool).await.unwrap();
        place(&st, &id, body("quota", "1tb", &code)).await;
        place(&st, &id, body("plan", "pro", &code)).await;
        place(&st, &id, body("plan", "pro", &code)).await;
        let used: i32 = sqlx::query_scalar("SELECT uses_count FROM promo_codes WHERE code = $1")
            .bind(&code)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap();
        teardown(&st, &id, &code).await;
        assert_eq!(used, 1, "only the one redemption that went through may count");
    }
}
