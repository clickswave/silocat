//! Account OTP verification with expiry + attempt limiting.
//!
//! The single-use `otp` column on `users` (email verification and password
//! reset) is a 6-digit code. Without a bound on attempts or lifetime it can be
//! brute-forced (900k space) into an account takeover. `consume` verifies and
//! invalidates a code and counts a failure in one statement. A code locks after
//! `MAX_ATTEMPTS` wrong guesses until a fresh one is issued, and the account
//! locks after `DAILY_MAX_FAILURES` wrong guesses in a day however many codes
//! were issued, because anyone who knows the address can have a new reset code
//! issued every 60 seconds.

use sqlx::{Pool, Postgres};

/// How long a freshly issued OTP stays valid.
pub const TTL_MINUTES: i64 = 10;
/// Wrong guesses allowed before the code is locked out.
pub const MAX_ATTEMPTS: i32 = 5;
/// Wrong guesses allowed per account in a 24 hour window, across all codes.
/// Ten a day against 900,000 codes is about a 0.4% chance in a year.
pub const DAILY_MAX_FAILURES: i32 = 10;

pub enum Outcome {
    Valid,
    Invalid,
}

/// The daily window has not started, or started over 24 hours ago.
const FRESH_DAY: &str =
    "(otp_day_started_at IS NULL OR otp_day_started_at <= NOW() - INTERVAL '24 hours')";

/// Verify and consume the account OTP for `user_id`. Succeeds only when the
/// code matches, is present and unexpired, and both the per-code and the daily
/// limits have room, then clears it. Otherwise both counters go up. It is a
/// single UPDATE, so parallel guesses queue on the row lock and each one is
/// judged on the counts the previous one left.
pub async fn consume(pool: &Pool<Postgres>, user_id: &str, provided: &str) -> Outcome {
    // Every SET expression reads the row as it was before this UPDATE, so the
    // verdict is spelled out in each one rather than computed once.
    let ok = format!(
        "(otp = $2 AND otp_expires_at IS NOT NULL AND otp_expires_at > NOW() \
         AND otp_attempts < $3 AND ({FRESH_DAY} OR otp_day_failures < $4))"
    );
    let sql = format!(
        "UPDATE users SET
           otp = CASE WHEN {ok} THEN '' ELSE otp END,
           otp_expires_at = CASE WHEN {ok} THEN NULL ELSE otp_expires_at END,
           otp_attempts = CASE WHEN {ok} THEN 0 ELSE otp_attempts + 1 END,
           otp_day_failures = CASE WHEN {ok} THEN 0
                                   WHEN {FRESH_DAY} THEN 1
                                   ELSE otp_day_failures + 1 END,
           otp_day_started_at = CASE WHEN {ok} THEN NULL
                                     WHEN {FRESH_DAY} THEN NOW()
                                     ELSE otp_day_started_at END
         WHERE id = $1 AND otp <> ''
         RETURNING otp = ''"
    );
    // Only a row with a code to check is touched; success is the only way the
    // code ends up empty.
    let consumed = sqlx::query_scalar::<_, bool>(&sql)
        .bind(user_id)
        .bind(provided)
        .bind(MAX_ATTEMPTS)
        .bind(DAILY_MAX_FAILURES)
        .fetch_optional(pool)
        .await;

    match consumed {
        Ok(Some(true)) => Outcome::Valid,
        Ok(_) => Outcome::Invalid,
        Err(e) => {
            println!("[OTP] consume failed: {:?}", e);
            Outcome::Invalid
        }
    }
}

#[cfg(test)]
mod tests {
    //! These run against a Postgres that has the api_switch migrations applied,
    //! named by SILOCAT_TEST_DATABASE_URL, and skip when it is unset. They need
    //! a pool, so each deletes the account it made.

    use super::*;
    use crate::libs::{apikey, rng};

    async fn pool() -> Option<Pool<Postgres>> {
        let url = std::env::var("SILOCAT_TEST_DATABASE_URL").ok()?;
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        Some(Pool::<Postgres>::connect(&url).await.expect("connect to SILOCAT_TEST_DATABASE_URL"))
    }

    async fn seed(pool: &Pool<Postgres>) -> String {
        let id = rng::uuid();
        let key = apikey::mint().expect("test key");
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
             api_key_migrated, account_type, email_verified) \
             VALUES ($1, $2, $3, 'hash', '', $4, $5, TRUE, 'personal', true)",
        )
        .bind(&id)
        .bind(format!("otp{}", rng::string(10)).to_lowercase())
        .bind(format!("{}@example.com", id))
        .bind(&key.blind_index)
        .bind(&key.encrypted)
        .execute(pool)
        .await
        .expect("seed user");
        id
    }

    /// What /user/forgot-password does to the row when it issues a code.
    async fn issue(pool: &Pool<Postgres>, id: &str, code: &str) {
        sqlx::query(
            "UPDATE users SET otp = $1, otp_last_sent_at = NOW(), \
             otp_expires_at = NOW() + INTERVAL '10 minutes', otp_attempts = 0 WHERE id = $2",
        )
        .bind(code)
        .bind(id)
        .execute(pool)
        .await
        .expect("issue code");
    }

    async fn cleanup(pool: &Pool<Postgres>, id: &str) {
        let _ = sqlx::query("DELETE FROM users WHERE id = $1").bind(id).execute(pool).await;
    }

    fn valid(o: Outcome) -> bool {
        matches!(o, Outcome::Valid)
    }

    #[tokio::test]
    async fn a_new_code_does_not_refill_the_daily_guess_budget() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        // An attacker who knows the address asks for a code every minute and
        // spends the per-code limit on each one.
        let mut wrong_accepted = 0;
        for round in 0..(DAILY_MAX_FAILURES / MAX_ATTEMPTS) {
            issue(&pool, &id, &format!("90000{}", round)).await;
            for guess in 0..MAX_ATTEMPTS {
                if valid(consume(&pool, &id, &format!("1{:05}", guess)).await) {
                    wrong_accepted += 1;
                }
            }
        }
        // The budget is spent: even the right code must not work today.
        issue(&pool, &id, "424242").await;
        let right_after_budget = valid(consume(&pool, &id, "424242").await);

        // A day later the owner can reset again.
        issue(&pool, &id, "535353").await;
        let _ = sqlx::query("UPDATE users SET otp_day_started_at = NOW() - INTERVAL '25 hours' WHERE id = $1")
            .bind(&id)
            .execute(&pool)
            .await;
        let right_next_day = valid(consume(&pool, &id, "535353").await);

        cleanup(&pool, &id).await;
        assert_eq!(wrong_accepted, 0);
        assert!(!right_after_budget, "the right code worked after the daily budget of wrong guesses");
        assert!(right_next_day, "the right code still failed after the window passed");
    }

    #[tokio::test]
    async fn the_right_code_works_once_and_wrong_ones_lock_the_code() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        issue(&pool, &id, "111111").await;
        let first = valid(consume(&pool, &id, "111111").await);
        let replay = valid(consume(&pool, &id, "111111").await);

        issue(&pool, &id, "222222").await;
        for _ in 0..MAX_ATTEMPTS {
            let _ = consume(&pool, &id, "000000").await;
        }
        let after_lock = valid(consume(&pool, &id, "222222").await);

        cleanup(&pool, &id).await;
        assert!(first, "the right code was refused");
        assert!(!replay, "a used code worked a second time");
        assert!(!after_lock, "the code still worked after the per-code limit");
    }

    #[tokio::test]
    async fn parallel_guesses_are_each_counted() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;
        issue(&pool, &id, "777777").await;

        let guesses: Vec<_> = (0..12)
            .map(|g| {
                let pool = pool.clone();
                let id = id.clone();
                tokio::spawn(async move { valid(consume(&pool, &id, &format!("3{:05}", g)).await) })
            })
            .collect();
        for g in guesses {
            assert!(!g.await.unwrap());
        }
        let (attempts, day): (i32, i32) =
            sqlx::query_as("SELECT otp_attempts, otp_day_failures FROM users WHERE id = $1")
                .bind(&id)
                .fetch_one(&pool)
                .await
                .expect("read counters");

        cleanup(&pool, &id).await;
        assert_eq!(attempts, 12, "per-code counter");
        assert_eq!(day, 12, "daily counter");
    }
}
