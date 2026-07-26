//! Account OTP verification with expiry + attempt limiting.
//!
//! The single-use `otp` column on `users` (email verification and password
//! reset) is a 6-digit code. Without a bound on attempts or lifetime it can be
//! brute-forced (900k space) into an account takeover. `consume` verifies and
//! invalidates a code atomically, counts failures, and locks out after
//! `MAX_ATTEMPTS` until a fresh code is issued (which resets the counter).

use sqlx::{Pool, Postgres};

/// How long a freshly issued OTP stays valid.
pub const TTL_MINUTES: i64 = 10;
/// Wrong guesses allowed before the code is locked out.
pub const MAX_ATTEMPTS: i32 = 5;

pub enum Outcome {
    Valid,
    Invalid,
}

/// Atomically verify and consume the account OTP for `user_id`. Succeeds only
/// when the code matches, is present, unexpired, and under the attempt limit: /// then clears it. On any failure the attempt counter is incremented (best
/// effort) so repeated guessing locks the code.
pub async fn consume(pool: &Pool<Postgres>, user_id: &str, provided: &str) -> Outcome {
    let consumed = sqlx::query_scalar::<_, String>(
        "UPDATE users
         SET otp = '', otp_attempts = 0, otp_expires_at = NULL
         WHERE id = $1
           AND otp <> '' AND otp = $2
           AND otp_expires_at IS NOT NULL AND otp_expires_at > NOW()
           AND otp_attempts < $3
         RETURNING id",
    )
    .bind(user_id)
    .bind(provided)
    .bind(MAX_ATTEMPTS)
    .fetch_optional(pool)
    .await;

    match consumed {
        Ok(Some(_)) => Outcome::Valid,
        _ => {
            // Count the failed attempt, but only while a code is actually active.
            let _ = sqlx::query(
                "UPDATE users SET otp_attempts = otp_attempts + 1 WHERE id = $1 AND otp <> ''",
            )
            .bind(user_id)
            .execute(pool)
            .await;
            Outcome::Invalid
        }
    }
}
