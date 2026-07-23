pub mod libs;
pub mod routes;
pub mod middlewares;
pub mod models;
pub mod watchcat;

use anyhow::Error;

/// Fail-fast config gate. If any required environment variable is *unset*, print
/// the full list and exit(1) before doing any work — never fall back to a silent
/// default for required config. Checks presence (unset), not emptiness, so a
/// feature can be intentionally left off (blank value) without blocking startup.
fn require_env(keys: &[&str]) {
    let missing: Vec<&str> = keys
        .iter()
        .copied()
        .filter(|k| std::env::var(k).is_err())
        .collect();
    if !missing.is_empty() {
        eprintln!(
            "FATAL: required environment variable(s) not set: {}\n\
             Refusing to start with missing configuration.",
            missing.join(", ")
        );
        std::process::exit(1);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub pg_pool: sqlx::Pool<sqlx::Postgres>,
    pub authority_sign: String,
    pub smtp_config: libs::configs::SmtpConfig,
    pub r2: libs::r2::R2,
    pub google_oauth: libs::configs::GoogleOauthConfig,
    pub geoip: Option<libs::geoip::GeoReader>,
    pub rate_limiter: libs::ratelimit::RateLimiter,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // WatchCat scheduler mode (separate container): needs only DB + R2.
    let watchcat_mode = std::env::var("WATCHCAT_MODE")
        .map(|v| matches!(v.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    // Fail fast if any required config is unset (before touching DB/R2).
    let mut required: Vec<&str> = vec![
        "DATABASE_URL",
        "CF_R2_SHADOW_API_URL",
        "CF_R2_SANCTUM_API_URL",
        "CF_R2_SHADOW_ACCESS_ID",
        "CF_R2_SANCTUM_ACCESS_ID",
        "CF_R2_SHADOW_ACCESS_SECRET",
        "CF_R2_SANCTUM_ACCESS_SECRET",
        "CF_R2_SHADOW_BUCKET",
        "CF_R2_SANCTUM_BUCKET",
    ];
    if !watchcat_mode {
        required.extend([
            "AUTHORITY_SIGN", "OAUTH_ID_GOOGLE", "OAUTH_SECRET_GOOGLE",
            "SMTP_ADDRESS", "SMTP_USERNAME", "SMTP_PASSWORD",
            "RAZORPAY_ID", "RAZORPAY_SECRET",
        ]);
    }
    require_env(&required);

    // establish a postgres connection + R2 (both modes need these)
    let pg_pool = libs::postgresql::pool().await?;
    let r2 = libs::r2::R2::new().await;

    if watchcat_mode {
        // Scheduler owns no migrations (api_switch runs them); just loop forever.
        watchcat::run(pg_pool, r2).await;
        return Ok(());
    }

    // run all migrations
    sqlx::migrate!("./migrations").run(&pg_pool).await?;

    // source authority sign from env
    let authority_sign = match std::env::var("AUTHORITY_SIGN"){
        Ok(var) => var,
        Err(_) => {
            return Err(Error::msg("AUTHORITY_SIGN is missing from the environment"))
        }
    };

    let smtp_config = libs::configs::smtp_config()?;
    let google_oauth = libs::configs::google_oauth_config()?;

    // GeoLite2-City reader (shared, read-only). No-ops if the db is absent.
    let geoip = libs::geoip::load();

    let state = AppState {
        pg_pool,
        authority_sign,
        smtp_config,
        r2,
        google_oauth,
        geoip,
        rate_limiter: libs::ratelimit::RateLimiter::new(),
    };

    // Surface the pricing mode: any APP_ENV that isn't prod/production charges
    // the ₹1/$1 test prices, so a misconfigured prod box is visible in the logs.
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "(unset)".to_string());
    let is_prod_pricing = matches!(app_env.trim().to_ascii_lowercase().as_str(), "prod" | "production" | "(unset)");
    println!(
        "[startup] APP_ENV={} — pricing: {}",
        app_env,
        if is_prod_pricing { "PRODUCTION (real prices)" } else { "TEST (nominal prices)" }
    );

    let router = routes::all(state.clone()).await.with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:31337").await?;

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
