pub mod order;

pub mod verify;
pub mod check_promo;
pub mod history;
pub mod usage;
pub mod razorpay_webhook;



use axum::Router;
use axum::routing::post;
use crate::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/order", post(order::handle))
        .route("/verify", post(verify::handle))
        .route("/check-promo", post(check_promo::handle))
        .route("/history", axum::routing::get(history::handle))
        .route("/usage", axum::routing::get(usage::handle))
        // Billing is user-scoped: require a valid user session (X-Api-Key) and
        // derive the acting user from the token, never from a body user_id.
        .route_layer(axum::middleware::from_fn_with_state(
            state,
            crate::middlewares::validate_token::validate_token,
        ))
}
