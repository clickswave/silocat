use crate::libs::rng;
use crate::models::UserTokenData;
use crate::routes::respond;
use crate::{libs, models};
use axum::{extract::State, response::IntoResponse, Json};
use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use sqlx::Row;

#[derive(Deserialize, Debug)]
pub struct RegisterAccountInput {
    pub username: String,
    pub email: String,
    pub password: String,
    pub invite_code: Option<String>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<RegisterAccountInput>,
) -> impl IntoResponse {
    // input validation
    let mut validation_errors = vec![];
    // validate email
    if let Err(errors) = libs::input_validators::email(&payload.email) {
        validation_errors.extend(errors);
    }
    // validate password
    if let Err(error) = libs::input_validators::password(&payload.password) {
        validation_errors.extend(error);
    }
    // return all the validation errors
    if validation_errors.len() > 0 {
        return respond(
            400,
            "Your input contains errors",
            validation_errors,
            json!({}),
        );
    }

    // validate invite code
    let valid_invite_code = match payload.invite_code {
        Some(ref code) => {
            let invite_code_exists = sqlx::query_as!(
                models::InviteCode,
                "
                SELECT * FROM invite_codes WHERE code = $1 AND account_type = 'personal' AND claimed_by IS NULL
                ",
                code
            ).fetch_optional(&axum_state.pg_pool).await;

            match invite_code_exists {
                Ok(invite_code) => invite_code,
                Err(_) => {
                    return respond(
                        500,
                        "Could not create a new account",
                        vec![
                            "Something went wrong while trying to verify your invite code"
                                .to_string(),
                        ],
                        json!({}),
                    );
                }
            }
        }
        None => None,
    };

    // Signup gate. Invite-only by default; set SILOCAT_INVITE_ONLY=false to open
    // public registration. Invites still grant their benefits when supplied —
    // they just stop being mandatory once signup is public.
    let invite_only = std::env::var("SILOCAT_INVITE_ONLY")
        .map(|v| {
            let v = v.trim().to_ascii_lowercase();
            !(v == "false" || v == "0" || v == "no")
        })
        .unwrap_or(true);

    if invite_only && valid_invite_code.is_none() {
        return respond(
            400,
            "Invite code required",
            vec!["This service is currently invite-only. Please provide a valid invite code.".to_string()],
            json!({}),
        );
    }

    // hash the password
    let password_hash = match libs::argon2::hash(payload.password) {
        Ok(hash) => hash,
        Err(_) => {
            return respond(
                500,
                "Could not create a new account",
                vec!["Something went wrong while trying to hash your password".to_string()],
                json!({}),
            );
        }
    };

    let otp = libs::rng::number(6);
    let user_id = rng::uuid();


    // Determine initial storage
    let mut initial_storage_bytes: i64 = 53687091200; // 50 GB default

    if let Some(ref invite) = valid_invite_code {
        if invite.benefit.ends_with("GB") {
            let gb_str = invite.benefit.trim_end_matches("GB");
            if let Ok(gb) = gb_str.parse::<i64>() {
                initial_storage_bytes = gb * 1024 * 1024 * 1024;
            }
        }
    }

    let user = sqlx::query_as::<_, models::User>(
        "
        INSERT INTO users
            (
             id,
             username,
             email,
             password_hash,
             otp,
             api_key,
             team_id,
             subscription_id,
             account_type,
             default_storage_bytes
            )
        VALUES
            ( $1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "
    )
    .bind(&user_id)
    .bind(&payload.username)
    .bind(&payload.email)
    .bind(password_hash)
    .bind(&otp)
    .bind(rng::uuid())
    .bind(None::<String>)
    .bind(None::<String>)
    .bind("personal")
    .bind(initial_storage_bytes)
    .fetch_one(&axum_state.pg_pool)
    .await;

    match user {
        Ok(user) => {
            let mut created_subscription: Option<models::Subscription> = None;

            // Apply Subscription Benefits
            if let Some(ref invite) = valid_invite_code {
                let duration_days = match invite.benefit.as_str() {
                    "1m-pro" => Some(30),
                    "3m-pro" => Some(90),
                    "6m-pro" => Some(180),
                    "1y-pro" => Some(365),
                    _ => None
                };

                if let Some(days) = duration_days {
                     // Create Pro Subscription
                    let expires_on = Utc::now() + chrono::Duration::days(days);
                    
                    // We use sqlx::query (not macro) to match the surrounding style for dynamic inserts without macro overhead if DB not present
                    let sub_res = sqlx::query("INSERT INTO subscriptions (name, additional_space, created_by, expires_on, invited) VALUES ('Pro', 1099511627776, $1, $2, TRUE) RETURNING *")
                        .bind(&user.id)
                        .bind(expires_on)
                        .fetch_optional(&axum_state.pg_pool)
                        .await;

                   if let Ok(Some(row)) = sub_res {
                        // We fetch as dynamic row then map manually or use query_as if struct allows. 
                        // Since we used raw query string, let's just get the fields we need. 
                        // Ideally we cast to Subscription model if possible, but Row is fine for ID.
                        // Actually, we need the full subscription object for the token.
                        
                        let sub_id: String = row.get("id");
                        
                        // Update Token Data Subscription
                        // We can construct the Subscription object from the row to return it
                        let subscription = models::Subscription {
                            id: sub_id.clone(),
                            name: row.get("name"),
                            additional_space: row.get("additional_space"),
                            created_by: row.get("created_by"),
                            created_on: row.get("created_on"),
                            expires_on: row.get("expires_on"),
                            invited: row.get("invited"),
                        };
                        
                        // Update User Record
                        let _ = sqlx::query!("UPDATE users SET subscription_id = $1 WHERE id = $2", sub_id, user.id)
                            .execute(&axum_state.pg_pool)
                            .await;
                            
                        // Store explicitly for token
                        created_subscription = Some(subscription);
                   }
                }
            }

            // send verification email
            if let Err(e) = libs::email::send_verification_email(
                &axum_state.smtp_config,
                &payload.username,
                &payload.email,
                &otp
            ).await {
                println!("Failed to send verification email to {}: {}", payload.email, e);
                // We typically don't fail the request here, but logging is essential.
            }

            // token struct
            let mut user_token_data = UserTokenData {
                id: user.id,
                username: user.username,
                email: user.email,
                email_verified: user.email_verified,
                profile_image: user.profile_image,
                password_set: true,
                subscription: created_subscription,
                api_key: user.api_key,
                account_type: user.account_type,
                default_storage_bytes: user.default_storage_bytes,
                country: user.country,
                bio: user.bio,
            };

            let code = match valid_invite_code {
                Some(invite_code) => invite_code.code,
                None => "invalid".to_string(),
            };

            // mark the invite code as claimed
            let _ = sqlx::query!(
                "
                UPDATE invite_codes SET claimed_by = $1 WHERE code = $2 AND claimed_by IS NULL
                ",
                user_id,
                code
            )
            .execute(&axum_state.pg_pool)
            .await;

            return respond(
                201,
                "Account registration was successful",
                vec![],
                json!({"user": user_token_data}),
            );
        }
        Err(error) => {
            let check_error = error.as_database_error();
            match check_error {
                None => {
                    return respond(
                        500,
                        "Could not create a new account",
                        vec!["Something went wrong while creating a new account".to_string()],
                        json!({}),
                    );
                }
                Some(db_error) => {
                    if db_error.is_unique_violation() {
                        return respond(
                            409,
                            "Could not create a new account",
                            vec![
                                "Username or email already exists, please choose a unique one"
                                    .to_string(),
                            ],
                            json!({}),
                        );
                    }
                }
            }

            if check_error.is_some() {
                if check_error.unwrap().is_unique_violation() {
                    return respond(
                        409,
                        "Could not create a new account",
                        vec!["User already exists, please choose a different email".to_string()],
                        json!({}),
                    );
                }
            }

            respond(
                500,
                "Could not create a new account",
                vec!["Something went wrong while creating a new account".to_string()],
                json!({}),
            )
        }
    };

    respond(
        500,
        "Could not create a new account",
        vec!["Something went wrong while creating a new account".to_string()],
        json!({}),
    )
}
