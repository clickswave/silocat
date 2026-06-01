use axum::{Json, response::IntoResponse};
use serde::{Deserialize};
use serde_json::json;
use crate::routes::respond;
use axum::extract::State;
use validator::Validate;

#[derive(Deserialize, Validate, Debug)]
pub struct Payload {
    #[validate(email)]
    pub email: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    if let Err(_) = payload.validate() {
        return respond(400, "Invalid email address", vec![], json!({}));
    }

    let insert_result = sqlx::query!(
        "INSERT INTO early_access_requests (email) VALUES ($1) ON CONFLICT (email) DO NOTHING",
        payload.email
    )
    .execute(&state.pg_pool)
    .await;

    match insert_result {
        Ok(res) => {
            if res.rows_affected() == 0 {
                respond(409, "You have already requested early access.", vec![], json!({}))
            } else {
                respond(200, "Request saved", vec![], json!({}))
            }
        }
        Err(e) => {
            println!("Error saving early access request: {:?}", e);
            respond(500, "Failed to save request", vec![e.to_string()], json!({}))
        }
    }
}
