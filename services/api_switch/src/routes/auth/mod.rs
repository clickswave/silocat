use axum::Router;

// The public early-access submission route was removed (the feature is retired,
// and it was the last user of the `validator` crate). Existing requests remain
// viewable through the admin surface. The /auth nest is kept as an empty mount
// point for future public auth routes.
pub fn router() -> Router<crate::AppState> {
    Router::new()
}
