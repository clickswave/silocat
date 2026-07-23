use crate::models::{Order, UserTokenData};
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension};
use serde_json::json;

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(token): Extension<UserTokenData>,
) -> impl IntoResponse {
    let orders_result = sqlx::query_as!(
        Order,
        "SELECT * FROM orders WHERE user_id = $1 ORDER BY created_on DESC",
        token.id
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
                vec![],
                json!({}),
            )
        }
    }
}
