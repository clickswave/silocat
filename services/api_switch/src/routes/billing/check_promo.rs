use axum::{extract::State, Json, response::IntoResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{routes::respond, models};

#[derive(Deserialize, Debug)]
pub struct CheckPromoPayload {
    pub code: String,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Json(payload): Json<CheckPromoPayload>,
) -> impl IntoResponse {

    let promo_res = sqlx::query_as!(
        models::PromoCode,
        "SELECT code, discount_percentage, duration, COALESCE(active, TRUE) as \"active!\" FROM promo_codes WHERE code = $1 AND active = TRUE",
        payload.code
    )
    .fetch_optional(&state.pg_pool)
    .await;

    match promo_res {
        Ok(Some(promo)) => {
            respond(200, "Promo code is valid", vec![], json!({
                "valid": true,
                "discount_percentage": promo.discount_percentage,
                "duration": promo.duration
            }))
        },
        Ok(None) => {
            respond(400, "Invalid or expired promo code", vec![], json!({
                "valid": false
            }))
        },
        Err(e) => {
             respond(500, "Database Error", vec![e.to_string()], json!({}))
        }
    }
}
