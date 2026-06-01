use axum::{extract::State, Json, response::IntoResponse, Router, routing::post};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{libs, models};
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", post(handle))
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
struct AdminTokenData {
    pub id: String,
    pub email: String,
    pub role: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // 1. Input Validation
    let mut validation_errors = vec![];
    if let Err(errors) = libs::input_validators::email(&payload.email) {
        validation_errors.extend(errors);
    }
    if validation_errors.len() > 0 {
        return respond(
            400,
            "Your input contains errors",
            validation_errors, json!({}),
        );
    }

    // 2. Fetch Admin User
    let find_admin_query = sqlx::query_as!(
        models::AdminUser,
        "SELECT * FROM admin_users WHERE email = $1",
        payload.email
    )
    .fetch_optional(&axum_state.pg_pool)
    .await;

    let admin = match find_admin_query {
        Ok(Some(admin)) => admin,
        Ok(None) => {
             return respond(
                401,
                "Authentication failed",
                vec!["Invalid credentials".to_string()],
                json!({}),
            );
        },
        Err(e) => {
            return respond(
                500,
                "Internal Server Error",
                vec![e.to_string()],
                json!({}),
            );
        }
    };

    // 3. Verify Password
    let is_valid = libs::argon2::verify(&payload.password, admin.password_hash);
    if !is_valid {
         return respond(
            401,
            "Authentication failed",
            vec!["Invalid credentials".to_string()],
            json!({}),
        );
    }

    // 4. Respond
    let response_data = AdminTokenData {
        id: admin.id,
        email: admin.email,
        role: admin.role,
    };

    respond(
        200,
        "Login successful",
        vec![],
        json!({"admin": response_data}),
    )
}
