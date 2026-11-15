use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, models, routes::respond};

#[derive(Deserialize, Debug)]
pub struct ResetPasswordInput {
    pub email: String,
    pub otp: String,
    pub new_password: String,
}

/// POST /user/reset-password  (public)
/// Verifies the emailed code and sets a new password. Proving control of the
/// inbox also verifies the email, so we mark it verified here. On success we
/// return fresh token data so the frontend can log the user straight in.
pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<ResetPasswordInput>,
) -> impl IntoResponse {
    // Per-IP throttle on top of the per-code attempt limit, so the OTP space
    // can't be swept from one host.
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !state.rate_limiter.check(&format!("reset:{}", ip), 15, std::time::Duration::from_secs(600)) {
        return respond(
            429,
            "Too Many Requests",
            vec!["Too many attempts. Please try again later.".to_string()],
            json!({}),
        );
    }
    // Validate the new password against our rules.
    if let Err(errors) = libs::input_validators::password(&payload.new_password) {
        return respond(400, "Invalid new password", errors, json!({}));
    }

    let email = payload.email.trim().to_string();
    let otp = payload.otp.trim().to_string();
    if email.is_empty() || otp.is_empty() {
        return respond(400, "Email and code are required", vec![], json!({}));
    }

    let user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE email = $1")
        .bind(&email)
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => {
            return respond(
                400,
                "Invalid or expired code",
                vec!["The code is incorrect or has expired.".to_string()],
                json!({}),
            )
        }
        Err(e) => {
            println!("[RESET_PW] lookup failed: {:?}", e);
            return respond(500, "Something went wrong", vec![], json!({}));
        }
    };

    // Verify + consume the reset code atomically: rejects an empty/expired code
    // and locks it after too many wrong guesses. It is a separate code from the
    // email verification one, so guesses here never touch that.
    match libs::otp::consume_reset(&state.pg_pool, &user.id, &otp).await {
        libs::otp::Outcome::Valid => {}
        libs::otp::Outcome::Invalid => {
            return respond(
                400,
                "Invalid or expired code",
                vec!["The code is incorrect or has expired.".to_string()],
                json!({}),
            );
        }
    }

    // Banned accounts get no session here either, the same refusal password
    // login gives. It comes after the code check so only the inbox owner
    // learns of the ban, as only the password holder does at login, and before
    // the claim so a refused reset leaves the password and key as they were.
    let ban_active = user.is_banned && user.banned_until.map(|u| u > chrono::Utc::now()).unwrap_or(true);
    if ban_active {
        return respond(
            403,
            "Account banned",
            vec![user.ban_reason.clone().unwrap_or_else(|| "Your account has been banned.".to_string())],
            json!({ "banned": true, "reason": user.ban_reason, "until": user.banned_until }),
        );
    }

    let new_hash = match libs::argon2::hash(payload.new_password) {
        Ok(h) => h,
        Err(_) => return respond(500, "Internal Error", vec!["Failed to process password".to_string()], json!({})),
    };

    // On an account nobody had verified, the code just proved who owns the
    // address, so whatever the registrant set goes too: key, share links,
    // username (see account_claim). The new key is in the token data returned
    // below.
    use super::account_claim::{self, Claim};
    let key = match libs::apikey::mint() {
        Some(k) => k,
        None => return respond(500, "Could not reset password", vec![], json!({})),
    };
    let reset = async {
        let mut tx = state.pg_pool.begin().await?;
        let claim = account_claim::password_reset(&mut tx, &user.id, &user.email, &new_hash, &key).await?;
        tx.commit().await?;
        Ok::<_, sqlx::Error>(claim)
    }
    .await;
    match reset {
        Ok(Claim::Moved) => {
            return respond(
                409,
                "Account changed",
                vec!["This account changed while the password was being reset. Please request a new code.".to_string()],
                json!({}),
            );
        }
        Ok(Claim::TakenOver) => {
            // The registrant's uploaded avatar, if any. The row no longer
            // points at it.
            let _ = state.r2.delete_object("dp", &user.id).await;
        }
        Ok(Claim::Verified) => {}
        Err(e) => {
            println!("[RESET_PW] update failed: {:?}", e);
            return respond(500, "Could not reset password", vec![], json!({}));
        }
    }

    // Return fresh token data (with the subscription) so the caller can sign in.
    let subscription = if let Some(sub_id) = &user.subscription_id {
        sqlx::query_as::<_, models::Subscription>("SELECT * FROM subscriptions WHERE id = $1 AND expires_on > NOW()")
            .bind(sub_id)
            .fetch_optional(&state.pg_pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let updated = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(&user.id)
        .fetch_one(&state.pg_pool)
        .await
    {
        Ok(u) => u,
        Err(_e) => return respond(500, "Failed to load profile", vec![], json!({})),
    };

    let token_data = models::token_data(updated, subscription);
    respond(200, "Password reset successful", vec![], json!({ "user": token_data }))
}

#[cfg(test)]
mod tests {
    //! These reset passwords on accounts in a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset. Each deletes the
    //! account it made.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::extract::ConnectInfo;

    const NEW_PASSWORD: &str = "Fresh-Password-42";

    /// What /user/forgot-password leaves on the row: a live reset code.
    async fn issue(st: &crate::AppState, id: &str) -> String {
        let code = libs::otp::new_reset_code();
        libs::otp::store_reset(&st.pg_pool, id, &code).await.unwrap();
        code
    }

    async fn reset(st: &crate::AppState, id: &str, otp: &str) -> (u16, serde_json::Value) {
        let email: String =
            sqlx::query_scalar("SELECT email FROM users WHERE id = $1").bind(id).fetch_one(&st.pg_pool).await.unwrap();
        let payload = ResetPasswordInput { email, otp: otp.to_string(), new_password: NEW_PASSWORD.to_string() };
        let resp = handle(ConnectInfo("127.0.0.1:1".parse().unwrap()), State(st.clone()), Default::default(), Json(payload))
            .await
            .into_response();
        let status = resp.status().as_u16();
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&body).unwrap())
    }

    async fn set(st: &crate::AppState, sql: &str, id: &str) {
        sqlx::query(sql).bind(id).execute(&st.pg_pool).await.expect(sql);
    }

    /// The columns a reset would change: password hash, key index, verified.
    async fn row(st: &crate::AppState, id: &str) -> (String, String, bool) {
        sqlx::query_as("SELECT password_hash, api_key, email_verified FROM users WHERE id = $1")
            .bind(id)
            .fetch_one(&st.pg_pool)
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn a_reset_signs_in_an_account_in_good_standing() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        let code = issue(&st, &id).await;
        let (status, body) = reset(&st, &id, &code).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 200);
        assert_eq!(body.pointer("/data/user/id"), Some(&json!(id)));
    }

    #[tokio::test]
    async fn a_banned_account_gets_no_session_from_a_reset() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NULL, ban_reason = 'abuse' WHERE id = $1", &id)
            .await;
        let before = row(&st, &id).await;
        let code = issue(&st, &id).await;
        let (status, body) = reset(&st, &id, &code).await;
        let after = row(&st, &id).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 403);
        assert!(body.pointer("/data/user").is_none(), "a refused reset must not carry the key");
        assert_eq!(body.pointer("/data/banned"), Some(&json!(true)));
        assert_eq!(after, before, "a refused reset must leave the password and key as they were");
    }

    #[tokio::test]
    async fn a_temporarily_banned_account_resets_once_the_ban_ends() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET is_banned = TRUE, banned_until = NOW() + INTERVAL '1 day' WHERE id = $1", &id).await;
        let code = issue(&st, &id).await;
        let (during, _) = reset(&st, &id, &code).await;
        set(&st, "UPDATE users SET banned_until = NOW() - INTERVAL '1 minute' WHERE id = $1", &id).await;
        let code = issue(&st, &id).await;
        let (after, _) = reset(&st, &id, &code).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(during, 403);
        assert_eq!(after, 200);
    }

    #[tokio::test]
    async fn a_banned_unverified_account_is_not_claimed_by_a_reset() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (id, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET email_verified = FALSE, is_banned = TRUE WHERE id = $1", &id).await;
        let before = row(&st, &id).await;
        let code = issue(&st, &id).await;
        let (status, _) = reset(&st, &id, &code).await;
        let after = row(&st, &id).await;
        forget(&st.pg_pool, &id).await;
        assert_eq!(status, 403);
        assert_eq!(after, before, "a refused reset must not claim the account");
    }

    #[tokio::test]
    async fn a_wrong_code_says_nothing_about_a_ban() {
        // Only someone holding the code learns the account is banned, as only
        // someone holding the password does at login.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (clean, _) = account(&st.pg_pool).await;
        let (banned, _) = account(&st.pg_pool).await;
        set(&st, "UPDATE users SET is_banned = TRUE, ban_reason = 'abuse' WHERE id = $1", &banned).await;
        issue(&st, &clean).await;
        issue(&st, &banned).await;
        let (clean_status, clean_body) = reset(&st, &clean, "WRONGCDE").await;
        let (banned_status, banned_body) = reset(&st, &banned, "WRONGCDE").await;
        forget(&st.pg_pool, &clean).await;
        forget(&st.pg_pool, &banned).await;
        assert_eq!(banned_status, clean_status);
        assert_eq!(banned_body, clean_body);
    }
}
