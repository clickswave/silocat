use axum::{extract::State, response::IntoResponse, Router, routing::get};
use serde_json::json;
use crate::{models};
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_orders))
}

async fn list_orders(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let orders = sqlx::query_as!(
        models::Order,
        "SELECT * FROM orders ORDER BY created_on DESC LIMIT 100"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match orders {
        Ok(orders) => respond(
            200,
            "Orders fetched successfully",
            vec![],
            json!({ "orders": orders }),
        ),
        Err(e) => respond(
            500,
            "Failed to fetch orders",
            vec![e.to_string()],
            json!({}),
        ),
    }
}
