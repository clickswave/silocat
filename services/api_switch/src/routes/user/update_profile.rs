use axum::{
    extract::{State, Json},
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use serde_json::json;
use crate::{models, routes::respond};

#[derive(Deserialize)]
pub struct UpdateProfilePayload {
    pub country: Option<String>,
    pub bio: Option<String>,
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
        Err(e) => return respond(500, "Database connection error", vec![e.to_string()], json!({})),
    };

    if let Some(bio) = payload.bio {
         if let Err(e) = sqlx::query(
            "UPDATE users SET bio = $1 WHERE id = $2"
        )
        .bind(bio)
        .bind(user.id.clone())
        .execute(&mut *tx)
        .await {
             let errors: Vec<String> = vec![e.to_string()];
             return respond(500, "Failed to update bio", errors, json!({}));
        }
    }

    if let Some(country) = payload.country {
        if let Err(e) = sqlx::query(
            "UPDATE users SET country = $1 WHERE id = $2"
        )
        .bind(country)
        .bind(user.id.clone())
        .execute(&mut *tx)
        .await {
            let errors = vec![e.to_string()];
            return respond(500, "Failed to update country", errors, json!({}));
        }
    }

    if let Err(e) = tx.commit().await {
        return respond(500, "Failed to commit changes", vec![e.to_string()], json!({}));
    }

    // Fetch updated user
    let updated_user = match sqlx::query_as::<_, models::User>("SELECT * FROM users WHERE id = $1")
        .bind(user.id.clone())
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(u)) => u,
        Ok(None) => return respond(404, "User not found after update", vec![], json!({})),
        Err(e) => return respond(500, "Failed to fetch updated profile", vec![e.to_string()], json!({})),
    };

    let token_data = models::token_data(updated_user, user.subscription.clone());

    respond(200, "Profile updated successfully", vec![], json!(token_data))
}
