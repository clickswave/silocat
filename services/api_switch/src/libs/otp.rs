//! Account codes with expiry and attempt limiting.
//!
//! Two codes live on `users`, each single use and emailed to the address.
//!
//! The email verification code (`otp`) is 6 digits. Only the signed-in account
//! can guess at it, through /user/verify-email, but that account may be one
//! somebody registered with an address they do not own, and it can have a new
//! code sent every 60 seconds. So `consume` locks a code after `MAX_ATTEMPTS`
//! wrong guesses and the account after `DAILY_MAX_FAILURES` wrong guesses in a
//! day, however many codes were issued.
//!
//! The password reset code (`reset_code`) can be guessed at by anyone who knows
//! the address. A per-account daily cap there would be a budget any stranger
//! could spend to keep the owner out, so instead the code is long enough that
//! guessing it is hopeless at any rate the mail limits allow, and only the
//! per-code limit applies. `consume_reset` never reads or touches the
//! verification code, and `consume` never touches the reset code.
//!
//! Each check and its failure count are one UPDATE, so parallel guesses queue
//! on the row lock and each is judged on the counts the previous one left.

use sqlx::{Pool, Postgres};

/// How long a freshly issued OTP stays valid.
pub const TTL_MINUTES: i64 = 10;
/// Wrong guesses allowed before the code is locked out.
pub const MAX_ATTEMPTS: i32 = 5;
/// Wrong guesses allowed per account in a 24 hour window, across all codes.
/// Ten a day against 900,000 codes is about a 0.4% chance in a year.
pub const DAILY_MAX_FAILURES: i32 = 10;

/// Password reset codes: `RESET_LEN` characters from `RESET_ALPHABET`, which
/// leaves out 0, 1, I, L, O and U so the code reads back unambiguously. That is
/// 30^8, about 6.6e11 codes. At the mail limits' 20 codes a day per address and
/// 5 guesses each it is about a 1 in 18 million chance in a year per account,
/// and spending the global mail cap of 2,000 codes a day on guesses across all
/// accounts gives about one success in 180,000 years.
pub const RESET_ALPHABET: &[u8] = b"23456789ABCDEFGHJKMNPQRSTVWXYZ";
pub const RESET_LEN: usize = 8;

pub enum Outcome {
    Valid,
    Invalid,
}

/// The daily window has not started, or started over 24 hours ago.
const FRESH_DAY: &str =
    "(otp_day_started_at IS NULL OR otp_day_started_at <= NOW() - INTERVAL '24 hours')";

/// Verify and consume the email verification code for `user_id`. Succeeds only
/// when the code matches, is present and unexpired, and both the per-code and
/// the daily limits have room, then clears it. Otherwise both counters go up. It is a
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

/// A new password reset code, from the thread CSPRNG.
pub fn new_reset_code() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..RESET_LEN)
        .map(|_| RESET_ALPHABET[rng.gen_range(0..RESET_ALPHABET.len())] as char)
        .collect()
}

/// The reset code as typed or pasted: any case, with spaces or hyphens.
pub fn normalize_reset_code(typed: &str) -> String {
    typed
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Make `code` the live reset code for `user_id`, with a fresh attempt count.
/// Called only once the email carrying it has gone out.
pub async fn store_reset(pool: &Pool<Postgres>, user_id: &str, code: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET reset_code = $1, reset_expires_at = NOW() + make_interval(mins => $2), \
         reset_attempts = 0 WHERE id = $3",
    )
    .bind(code)
    .bind(TTL_MINUTES as i32)
    .bind(user_id)
    .execute(pool)
    .await
    .map(|_| ())
}

/// Verify and consume the password reset code for `user_id`: it must match, be
/// unexpired and have had fewer than `MAX_ATTEMPTS` wrong guesses. A wrong
/// guess counts against this code only.
pub async fn consume_reset(pool: &Pool<Postgres>, user_id: &str, typed: &str) -> Outcome {
    let ok = "(reset_code = $2 AND reset_expires_at IS NOT NULL AND reset_expires_at > NOW() \
              AND reset_attempts < $3)";
    let sql = format!(
        "UPDATE users SET
           reset_code = CASE WHEN {ok} THEN '' ELSE reset_code END,
           reset_expires_at = CASE WHEN {ok} THEN NULL ELSE reset_expires_at END,
           reset_attempts = CASE WHEN {ok} THEN 0 ELSE reset_attempts + 1 END
         WHERE id = $1 AND reset_code <> ''
         RETURNING reset_code = ''"
    );
    let consumed = sqlx::query_scalar::<_, bool>(&sql)
        .bind(user_id)
        .bind(normalize_reset_code(typed))
        .bind(MAX_ATTEMPTS)
        .fetch_optional(pool)
        .await;

    match consumed {
        Ok(Some(true)) => Outcome::Valid,
        Ok(_) => Outcome::Invalid,
        Err(e) => {
            println!("[OTP] consume_reset failed: {:?}", e);
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

    /// What /user/resend-verification does to the row when it issues a code.
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

        // Whoever holds the session (perhaps someone who registered an address
        // they do not own) has a code sent every minute and spends the
        // per-code limit on each one.
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

        // A day later the account can verify again.
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
    async fn parallel_guesses_each_raise_both_counters() {
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

    /// What /user/forgot-password does to the row once the email has gone out.
    async fn issue_reset(pool: &Pool<Postgres>, id: &str, code: &str) {
        store_reset(pool, id, code).await.expect("store reset code");
    }

    #[tokio::test]
    async fn strangers_guessing_reset_codes_cannot_lock_the_owner_out() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        // Anyone who knows the address can have codes sent and spend the
        // per-code limit on each, well past any daily cap.
        let mut wrong_accepted = 0;
        for round in 0..3 {
            issue_reset(&pool, &id, &format!("ZZZZZZZ{}", round + 2)).await;
            for guess in 0..MAX_ATTEMPTS {
                if valid(consume_reset(&pool, &id, &format!("YYYYYYY{}", guess + 2)).await) {
                    wrong_accepted += 1;
                }
            }
        }
        // The owner asks for a code of their own and it still works.
        let code = new_reset_code();
        issue_reset(&pool, &id, &code).await;
        let owner = valid(consume_reset(&pool, &id, &code).await);

        cleanup(&pool, &id).await;
        assert_eq!(wrong_accepted, 0);
        assert!(owner, "wrong reset guesses by others locked the owner out");
    }

    #[tokio::test]
    async fn reset_guesses_leave_the_verification_code_alone() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        // A new signup waits on its verification code while a stranger guesses
        // at the public reset endpoint.
        issue(&pool, &id, "246810").await;
        issue_reset(&pool, &id, &new_reset_code()).await;
        for guess in 0..(DAILY_MAX_FAILURES + MAX_ATTEMPTS) {
            let _ = consume_reset(&pool, &id, &format!("{:06}", guess)).await;
        }
        let verified = valid(consume(&pool, &id, "246810").await);

        cleanup(&pool, &id).await;
        assert!(verified, "reset guesses used up the verification code's attempts");
    }

    #[tokio::test]
    async fn a_verification_code_does_not_reset_a_password() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        // Six digits that whoever registered the account can have sent every
        // minute must not open the reset path, which has no daily cap.
        issue(&pool, &id, "135791").await;
        let reset_with_verification = valid(consume_reset(&pool, &id, "135791").await);

        cleanup(&pool, &id).await;
        assert!(!reset_with_verification, "a verification code reset the password");
    }

    #[tokio::test]
    async fn a_reset_code_works_once_however_it_is_typed() {
        let Some(pool) = pool().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let id = seed(&pool).await;

        issue_reset(&pool, &id, "ABCD2345").await;
        let typed = valid(consume_reset(&pool, &id, " abcd-2345 ").await);
        let replay = valid(consume_reset(&pool, &id, "ABCD2345").await);

        cleanup(&pool, &id).await;
        assert!(typed, "the right code typed in lower case with a hyphen was refused");
        assert!(!replay, "a used reset code worked a second time");
    }

    #[test]
    fn reset_codes_are_too_long_to_guess() {
        let codes: Vec<String> = (0..200).map(|_| new_reset_code()).collect();
        for c in &codes {
            assert_eq!(c.len(), RESET_LEN, "{c}");
            assert!(c.bytes().all(|b| RESET_ALPHABET.contains(&b)), "{c}");
        }
        let distinct: std::collections::HashSet<_> = codes.iter().collect();
        assert!(distinct.len() > 190, "codes repeat");
        // At 100 guesses a day for a year, at most 1 in a million.
        let space = (RESET_ALPHABET.len() as f64).powi(RESET_LEN as i32);
        assert!(space > 100.0 * 365.0 * 1e6, "only {space} reset codes");
    }
}
