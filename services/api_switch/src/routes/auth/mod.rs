pub mod early_access;

use axum::routing::post;
use axum::Router;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/early-access", post(early_access::handle))
}
