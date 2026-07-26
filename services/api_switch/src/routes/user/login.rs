use axum::{extract::State, Json, response::IntoResponse};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json};
use crate::{libs, models};
use crate::routes::respond;

/// Verified against when the email is unknown, so a missing account costs the
/// same time as a wrong password and cannot be detected by timing.
///
/// The salt and digest are random bytes: this hash corresponds to no password
/// that has ever existed, and cracking it yields nothing. That matters, because
/// this constant previously held the hash of a real account's password, which
/// meant the credential shipped in the source even after it was removed from
/// the migration that first introduced it.
///
/// Argon2 verification does the full KDF work before comparing, so a digest
/// that matches nothing equalises timing exactly as a real one would.
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$kcAIvWgAXrW1BuRgpYA/qw$H8fMKKi/uocv5jEa6IXKk0L7eADTErOBQyi3DrClIoI";

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub email: String,
    pub password: String,
}

pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Per-IP throttle to blunt credential stuffing / password brute-force.
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !axum_state.rate_limiter.check(&format!("login:{}", ip), 10, std::time::Duration::from_secs(300)) {
        return respond(
            429,
            "Too Many Requests",
            vec!["Too many attempts. Please try again in a few minutes.".to_string()],
            json!({}),
        );
    }

    // input validation
    let mut validation_errors = vec![];
    // validate email
    if let Err(errors) = libs::input_validators::email(&payload.email) {
        validation_errors.extend(errors);
    }
    // return all the validation errors
    if validation_errors.len() > 0 {
        return respond(
            400,
            "Your input contains errors",
            validation_errors, json!({}),
        );
    }
    let find_user_query =  sqlx::query_as::<_, models::User>(
        "SELECT * FROM users WHERE email = $1"
    )
    .bind(&payload.email)
    .fetch_one(&axum_state.pg_pool).await;

    let user = match find_user_query {
        Ok(user) => user,
        Err(sqlx::Error::RowNotFound) => {
            // Run a verify against a dummy hash so an unknown email takes the
            // same time as a wrong password: no email enumeration by timing.
            let _ = libs::argon2::verify(&payload.password, DUMMY_HASH.to_string());
            return respond(
                401,
                "Could not login",
                vec!["Email and password combination is invalid".to_string()],
                json!({}),
            );
        }
        Err(_) => {
            return respond(
                500,
                "Could not login",
                vec!["Something went wrong while trying to login".to_string()],
                json!({}),
            );
        }
    };

    // Banned accounts can't log in.
    let ban_active =
        user.is_banned && user.banned_until.map(|u| u > Utc::now()).unwrap_or(true);
    if ban_active {
        return respond(
            403,
            "Account banned",
            vec![user
                .ban_reason
                .clone()
                .unwrap_or_else(|| "Your account has been banned.".to_string())],
            json!({ "banned": true, "reason": user.ban_reason, "until": user.banned_until }),
        );
    }

    let find_subscription_query = sqlx::query_as!(models::Subscription,
        "
        SELECT * FROM subscriptions \
        WHERE id = $1 AND expires_on > NOW() \
        ORDER BY created_on DESC
        ",
        user.subscription_id
    ).fetch_optional(&axum_state.pg_pool).await;

    let subscription = match find_subscription_query {
        Ok(subscription) => subscription,
        Err(_) => {
            return respond(
                500,
                "Could not login",
                vec!["Something went wrong while trying to fetch your subscription".to_string()],
                json!({}),
            );
        }
    };

    let check_password = libs::argon2::verify(&payload.password, user.password_hash);

    match check_password {
        true => {
            let user_token_data = models::UserTokenData {
                id: user.id.to_string(),
                username: user.username,
                email: user.email,
                profile_image: user.profile_image,
                email_verified: user.email_verified,
                password_set: true,
                subscription,
                api_key: user.api_key,
                account_type: user.account_type,
                default_storage_bytes: user.default_storage_bytes,
                country: user.country,
                bio: user.bio,
            };

            respond(
                200,
                "Welcome back",
                vec![],
                json!({"user": user_token_data}),
            )
        }
        false => {
            respond(
                401,
                "Could not login",
                vec!["Email and password combination is invalid".to_string()],
                json!({}),
            )
        }
    }
}
