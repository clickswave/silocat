use axum::{extract::State, Json, response::IntoResponse, Router, routing::{get, post}};
use serde::{Deserialize};
use serde_json::json;
use crate::{models};
use crate::routes::respond;

#[derive(Deserialize)]
pub struct CreatePromoPayload {
    pub code: String,
    pub discount_percentage: i32,
    pub duration: String,
    pub active: bool,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_promos).post(create_promo))
}

async fn list_promos(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let promos = sqlx::query_as!(
        models::PromoCode,
        r#"SELECT code, discount_percentage, duration, active as "active!" FROM promo_codes ORDER BY code ASC"#
    )
    .fetch_all(&state.pg_pool)
    .await;

    match promos {
        Ok(promos) => respond(
            200,
            "Promos fetched successfully",
            vec![],
            json!({ "promos": promos }),
        ),
        Err(e) => respond(
            500,
            "Failed to fetch promos",
            vec![e.to_string()],
            json!({}),
        ),
    }
}

async fn create_promo(
    State(state): State<crate::AppState>,
    Json(payload): Json<CreatePromoPayload>,
) -> impl IntoResponse {
    let result = sqlx::query!(
        "INSERT INTO promo_codes (code, discount_percentage, duration, active) VALUES ($1, $2, $3, $4) RETURNING *",
        payload.code,
        payload.discount_percentage,
        payload.duration,
        payload.active
    )
    .fetch_one(&state.pg_pool)
    .await;

    match result {
        Ok(record) => respond(
            200,
            "Promo created successfully",
            vec![],
            json!({ 
                "promo": {
                    "code": record.code,
                    "discount_percentage": record.discount_percentage,
                    "duration": record.duration,
                    "active": record.active
                } 
            }),
        ),
        Err(e) => respond(
            500,
            "Failed to create promo",
            vec![e.to_string()],
            json!({}),
        ),
    }
}
