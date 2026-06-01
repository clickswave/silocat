pub mod order;

pub mod verify;
pub mod check_promo;
pub mod history;
pub mod usage;



use axum::Router;
use axum::routing::post;
use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/order", post(order::handle))

        .route("/verify", post(verify::handle))
        .route("/check-promo", post(check_promo::handle))
        .route("/history", axum::routing::get(history::handle))
        .route("/usage", axum::routing::get(usage::handle))
}
