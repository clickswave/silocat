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
        // Order history is a list of receipts, so it carries settled orders only.
        // An abandoned checkout is not a purchase: showing it invites the reader
        // to wonder whether they were charged, and watchcat deletes it shortly.
        // order_is_settled() is the shared definition, also used by the
        // invoice-number trigger and watchcat's cleanup.
        "SELECT * FROM orders WHERE user_id = $1 AND order_is_settled(status) \
         ORDER BY created_on DESC",
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
