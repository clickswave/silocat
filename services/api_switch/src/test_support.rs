//! Shared setup for tests that call handlers against a Postgres with the
//! api_switch migrations applied, named by SILOCAT_TEST_DATABASE_URL. Callers
//! skip when it is unset. The storage clients point at a closed local port, so
//! nothing a test does can reach a real bucket; presigning is local and works.

use crate::libs::{apikey, rng};
use sqlx::{Pool, Postgres};

const STORAGE_ENV: [&str; 9] = [
    "CF_R2_SHADOW_API_URL",
    "CF_R2_SHADOW_ACCESS_ID",
    "CF_R2_SHADOW_ACCESS_SECRET",
    "CF_R2_SANCTUM_API_URL",
    "CF_R2_SANCTUM_ACCESS_ID",
    "CF_R2_SANCTUM_ACCESS_SECRET",
    "CF_R2_DP_API_URL",
    "CF_R2_DP_ACCESS_ID",
    "CF_R2_DP_ACCESS_SECRET",
];

pub async fn state() -> Option<crate::AppState> {
    let url = std::env::var("SILOCAT_TEST_DATABASE_URL").ok()?;
    let pg_pool = Pool::<Postgres>::connect(&url).await.expect("connect to SILOCAT_TEST_DATABASE_URL");
    Some(state_with(pg_pool).await)
}

/// State for handlers a test drives only up to the storage call, which fails
/// against the closed port. The pool never connects, so no database is needed.
pub async fn offline_state() -> crate::AppState {
    let pg_pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://silocat@127.0.0.1:9/none")
        .expect("lazy pool");
    state_with(pg_pool).await
}

async fn state_with(pg_pool: Pool<Postgres>) -> crate::AppState {
    unsafe {
        std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests");
        for name in STORAGE_ENV {
            let value = if name.ends_with("_API_URL") { "http://127.0.0.1:9" } else { "test" };
            std::env::set_var(name, value);
        }
    }
    crate::AppState {
        pg_pool,
        smtp_config: crate::libs::configs::SmtpConfig {
            address: String::new(),
            username: String::new(),
            password: String::new(),
            from_name: String::new(),
            from_email: String::new(),
            reply_to_name: String::new(),
            reply_to_email: String::new(),
            use_ses_templates: false,
            ses_region: String::new(),
            template_prefix: String::new(),
        },
        r2: crate::libs::r2::R2::new().await,
        google_oauth: crate::libs::configs::GoogleOauthConfig {
            client_id: String::new(),
            client_secret: String::new(),
        },
        geoip: None,
        rate_limiter: crate::libs::ratelimit::RateLimiter::new(),
    }
}

/// A verified account as signup leaves it. Returns its id and key index.
pub async fn account(pool: &Pool<Postgres>) -> (String, String) {
    let id = rng::uuid();
    let key = apikey::mint().expect("test key");
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
         api_key_migrated, account_type, email_verified, default_storage_bytes) \
         VALUES ($1, $2, $3, 'hash', '', $4, $5, TRUE, 'personal', true, $6)",
    )
    .bind(&id)
    .bind(format!("t{}", rng::string(12)).to_lowercase())
    .bind(format!("{}@example.com", id))
    .bind(&key.blind_index)
    .bind(&key.encrypted)
    .bind(crate::libs::quota::signup_storage_bytes())
    .execute(pool)
    .await
    .expect("seed user");
    (id, key.blind_index)
}

/// Put the account on a plan that runs for another month.
pub async fn subscribe(pool: &Pool<Postgres>, user_id: &str) {
    let sub = rng::uuid();
    sqlx::query(
        "INSERT INTO subscriptions (id, name, additional_space, created_by, expires_on) \
         VALUES ($1, 'Plus', 0, $2, NOW() + INTERVAL '30 days')",
    )
    .bind(&sub)
    .bind(user_id)
    .execute(pool)
    .await
    .expect("seed subscription");
    sqlx::query("UPDATE users SET subscription_id = $1 WHERE id = $2")
        .bind(&sub)
        .bind(user_id)
        .execute(pool)
        .await
        .expect("link subscription");
}

/// Remove an account and everything these tests make for it.
pub async fn forget(pool: &Pool<Postgres>, user_id: &str) {
    for sql in [
        "DELETE FROM chunks WHERE file_id IN (SELECT id FROM files WHERE user_id = $1)",
        "DELETE FROM request_uploads WHERE request_id IN (SELECT id FROM file_requests WHERE owner_user_id = $1)",
        "DELETE FROM files WHERE user_id = $1",
        "DELETE FROM file_requests WHERE owner_user_id = $1",
        "DELETE FROM subscriptions WHERE created_by = $1",
        "DELETE FROM users WHERE id = $1",
    ] {
        sqlx::query(sql).bind(user_id).execute(pool).await.expect(sql);
    }
}

/// Storage that says every object it is asked about holds `size` bytes, and
/// accepts any delete. Lets a test mark a chunk complete, which the closed
/// port of `state()` never allows. The listener lives for the rest of the run.
pub fn storage_holding(size: u64) -> crate::libs::r2::R2 {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind fake storage");
    let endpoint = format!("http://{}", listener.local_addr().expect("fake storage address"));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let mut seen = Vec::new();
                while !seen.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf) {
                        Ok(0) | Err(_) => return,
                        Ok(n) => seen.extend_from_slice(&buf[..n]),
                    }
                }
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    if seen.starts_with(b"HEAD ") { size } else { 0 }
                );
                let _ = stream.write_all(head.as_bytes());
            });
        }
    });
    crate::libs::r2::R2::at(&endpoint)
}
