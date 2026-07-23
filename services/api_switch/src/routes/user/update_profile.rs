use axum::{
    extract::{State, Json},
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, models, routes::respond};

#[derive(Deserialize)]
pub struct UpdateProfilePayload {
    pub country: Option<String>,
    pub bio: Option<String>,
    pub username: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<models::UserTokenData>,
    Json(payload): Json<UpdateProfilePayload>,
) -> impl IntoResponse {
    
    // 1. Validation (Basic)
    if let Some(ref c) = payload.country {
        if c.len() != 2 {
            return respond(400, "Invalid country code", vec![], json!({}));
        }
    }

    // 2. Build Query
    let mut tx = match state.pg_pool.begin().await {
        Ok(tx) => tx,
        Err(_e) => return respond(500, "Database connection error", vec![], json!({})),
    };

    // Username is display-only here (no links/lookups depend on it). Validate it,
    // enforce "at most 2 changes per rolling 30 days", and rely on the UNIQUE
    // constraint to reject collisions with a clean 409. An early return drops
    // `tx`, rolling back any other writes -> atomic.
    if let Some(ref raw) = payload.username {
        let uname = raw.trim().to_string();
        if !uname.is_empty() {
            if let Err(errors) = libs::input_validators::username(&uname) {
                return respond(400, "Invalid username", errors, json!({}));
            }

            let row = sqlx::query_as::<_, (String, i32, Option<chrono::DateTime<chrono::Utc>>)>(
                "SELECT username, username_change_count, username_change_window_start \
                 FROM users WHERE id = $1",
            )
            .bind(user.id.clone())
            .fetch_one(&mut *tx)
            .await;

            let (current_username, count, window_start) = match row {
                Ok(r) => r,
                Err(_e) => return respond(500, "Failed to read profile", vec![], json!({})),
            };

            // Only enforce/record when the name actually changes.
            if uname != current_username {
                const LIMIT: i32 = 2;
                let now = chrono::Utc::now();
                let window_active = window_start
                    .map(|w| now.signed_duration_since(w).num_days() < 30)
                    .unwrap_or(false);

                let (new_count, new_window) = if window_active {
                    if count >= LIMIT {
                        let next = window_start.unwrap() + chrono::Duration::days(30);
                        return respond(
                            429,
                            "Username change limit reached",
                            vec![format!(
                                "You can change your username at most {} times a month. You can change it again on {}.",
                                LIMIT,
                                next.format("%b %d, %Y")
                            )],
                            json!({ "next_change_at": next.to_rfc3339() }),
                        );
                    }
                    (count + 1, window_start.unwrap())
                } else {
                    // Window expired (or first ever change): start a fresh one.
                    (1, now)
                };

                match sqlx::query(
                    "UPDATE users SET username = $1, username_change_count = $2, \
                     username_change_window_start = $3 WHERE id = $4",
                )
                .bind(&uname)
                .bind(new_count)
                .bind(new_window)
                .bind(user.id.clone())
                .execute(&mut *tx)
                .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        if e.as_database_error().map(|db| db.is_unique_violation()).unwrap_or(false) {
                            return respond(
                                409,
                                "Username taken",
                                vec!["That username is already in use, please choose another.".to_string()],
                                json!({}),
                            );
                        }
                        return respond(500, "Failed to update username", vec![], json!({}));
                    }
                }
            }
        }
    }

    if let Some(bio) = payload.bio {
         if let Err(_e) = sqlx::query(
            "UPDATE users SET bio = $1 WHERE id = $2"
        )
        .bind(bio)
        .bind(user.id.clone())
        .execute(&mut *tx)
        .await {
             let errors: Vec<String> = vec![];
             return respond(500, "Failed to update bio", errors, json!({}));
        }
    }

    if let Some(country) = payload.country {
        if let Err(_e) = sqlx::query(
            "UPDATE users SET country = $1 WHERE id = $2"
        )
        .bind(country)
        .bind(user.id.clone())
        .execute(&mut *tx)
        .await {
            let errors = vec![];
            return respond(500, "Failed to update country", errors, json!({}));
        }
    }

    if let Err(_e) = tx.commit().await {
        return respond(500, "Failed to commit changes", vec![], json!({}));
    }

    // Fetch updated user
    let updated_user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(user.id.clone())
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return respond(404, "User not found after update", vec![], json!({})),
        Err(_e) => return respond(500, "Failed to fetch updated profile", vec![], json!({})),
    };

    let token_data = models::token_data(updated_user, user.subscription.clone());

    respond(200, "Profile updated successfully", vec![], json!(token_data))
}
