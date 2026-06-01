pub mod libs;
pub mod routes;
pub mod middlewares;
pub mod models;

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    // Fail fast if any required config is unset (before touching DB/R2).
    require_env(&[
        "AUTHORITY_SIGN",
        "DATABASE_URL",
        "OAUTH_ID_GOOGLE",
        "OAUTH_SECRET_GOOGLE",
        "SMTP_ADDRESS",
        "SMTP_USERNAME",
        "SMTP_PASSWORD",
        "RAZORPAY_ID",
        "RAZORPAY_SECRET",
        "CF_R2_SHADOW_API_URL",
        "CF_R2_SANCTUM_API_URL",
        "CF_R2_SHADOW_ACCESS_ID",
        "CF_R2_SANCTUM_ACCESS_ID",
        "CF_R2_SHADOW_ACCESS_SECRET",
        "CF_R2_SANCTUM_ACCESS_SECRET",
        "CF_R2_SHADOW_BUCKET",
        "CF_R2_SANCTUM_BUCKET",
    ]);

    // source authority sign from env
    let authority_sign = match std::env::var("AUTHORITY_SIGN"){
        Ok(var) => var,
        Err(_) => {
            return Err(Error::msg("AUTHORITY_SIGN is missing from the environment"))
        }
    };

    // establish a postgres connection
    let pg_pool = libs::postgresql::pool().await?;

    // run all migrations
    sqlx::migrate!("./migrations").run(&pg_pool).await?;

    let smtp_config = libs::configs::smtp_config()?;
    let google_oauth = libs::configs::google_oauth_config()?;

    let r2 = libs::r2::R2::new().await;

    let state = AppState {
        pg_pool,
        authority_sign,
        smtp_config,
        r2,
        google_oauth,
    };

    let router = routes::all(state.clone()).await.with_state(state.clone());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:31337").await?;

    axum::serve(listener, router).await?;
    Ok(())
}
