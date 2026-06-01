use axum::{extract::State, response::IntoResponse, Router, routing::get};
use serde_json::json;
use crate::{models};
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_subscriptions))
}

async fn list_subscriptions(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let subscriptions = sqlx::query_as!(
        models::Subscription,
        "SELECT * FROM subscriptions ORDER BY created_on DESC LIMIT 100"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match subscriptions {
        Ok(subs) => respond(
            200,
            "Subscriptions fetched successfully",
            vec![],
            json!({ "subscriptions": subs }),
        ),
        Err(e) => respond(
            500,
            "Failed to fetch subscriptions",
            vec![e.to_string()],
            json!({}),
        ),
    }
}
