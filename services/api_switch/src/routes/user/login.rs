use axum::{extract::State, Json, response::IntoResponse};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json};
use crate::{libs, models};
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub email: String,
    pub password: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
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


    let find_subscription_query = sqlx::query_as!(models::Subscription,
        "
        SELECT * FROM subscriptions \
        WHERE id = $1 \
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
