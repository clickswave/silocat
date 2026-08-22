//! One canonical source of truth for what each paid plan grants and how long a
//! billing cycle lasts.
//!
//! This used to be copy-pasted in four places (order.rs, verify.rs,
//! razorpay_webhook.rs, register_personal.rs) with a comment on each begging the
//! others to stay in sync. They had already drifted: invite-Pro granted 1 TiB
//! while billing Pro grants 2 TB. Everything now delegates here.

/// Bytes in a gibibyte / tebibyte, so grants read as sizes rather than magic ints.
pub const GIB: i64 = 1024 * 1024 * 1024;
pub const TIB: i64 = 1024_i64.pow(4);

/// Paid plan identifier -> (display name stored on the subscription row, bytes
/// of additional storage). `None` for anything that is not a known paid plan.
pub fn plan_grant(identifier: &str) -> Option<(&'static str, i64)> {
    match identifier {
        "plus" => Some(("Plus", 200 * GIB)),
        "pro" => Some(("Pro", 2 * TIB)),
        _ => None,
    }
}

/// Bytes a paid plan adds (0 for an unknown identifier).
pub fn plan_space(identifier: &str) -> i64 {
    plan_grant(identifier).map(|(_, bytes)| bytes).unwrap_or(0)
}

/// Display name for a plan identifier.
pub fn plan_name(identifier: &str) -> Option<&'static str> {
    plan_grant(identifier).map(|(name, _)| name)
}

/// Postgres INTERVAL literal for a billing cycle. Chosen from a fixed set (never
/// interpolated from user input), so it is safe to inline into a query string.
pub fn cycle_interval(cycle: &str) -> &'static str {
    if cycle == "annual" { "1 year" } else { "1 month" }
}

/// Number of billing cycles a subscription runs before Razorpay stops it. Set
/// high so the mandate effectively renews "forever" until the user cancels; the
/// customer can cancel any time and our webhook stops extending the plan.
pub const SUBSCRIPTION_TOTAL_COUNT: u32 = 120; // 10 years of monthly, or annual

/// True when the user has any active (non-expired) subscription, i.e. a paid or
/// granted plan (Plus/Pro/invite/trial). Used to gate the delivery-workflow
/// features (delivery receipts, request-a-file), which are the paid monetization
/// surface; every privacy feature (encryption, link passwords, expiry, basic and
/// one-time sharing) stays free for everyone.
pub async fn is_paid(pool: &sqlx::Pool<sqlx::Postgres>, user_id: &str) -> bool {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM subscriptions WHERE created_by = $1 AND expires_on > NOW()",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
    .map(|c| c > 0)
    .unwrap_or(false)
}

/// The Razorpay Plan id for a (plan identifier, cycle, currency), read from the
/// environment. Plans are created out of band, once, against the Razorpay
/// account (see `scripts/silocat_razorpay_plans.py`), and their ids are wired in
/// via env vars named `RAZORPAY_PLAN_{PLAN}_{CYCLE}_{CURRENCY}`, e.g.
/// `RAZORPAY_PLAN_PRO_MONTHLY_INR`. Returns `None` when a plan/cycle/currency
/// combination has no configured plan (caller should fall back or 400).
pub fn razorpay_plan_id(identifier: &str, cycle: &str, currency: &str) -> Option<String> {
    let plan = identifier.to_uppercase();
    let cyc = if cycle == "annual" { "ANNUAL" } else { "MONTHLY" };
    let cur = currency.to_uppercase();
    let var = format!("RAZORPAY_PLAN_{}_{}_{}", plan, cyc, cur);
    std::env::var(var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}
