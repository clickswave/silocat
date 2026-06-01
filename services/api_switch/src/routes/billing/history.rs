use crate::models::{Order};
use crate::routes::respond;
use axum::{extract::{State, Query}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct HistoryParams {
    pub user_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Query(params): Query<HistoryParams>,
) -> impl IntoResponse {
    let orders_result = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE user_id = $1 ORDER BY created_on DESC",
        params.user_id
    )
    .fetch_all(&axum_state.pg_pool)
    .await;

    match orders_result {
        Ok(orders) => respond(
            200,
            "Order history fetched successfully",
            vec![],
            json!({ "orders": orders }),
        ),
        Err(e) => {
            println!("Failed to fetch orders: {}", e);
            respond(
                500,
                "Failed to fetch order history",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
