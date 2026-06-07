use axum::Router;
// use axum::middleware::from_fn;
// use crate::middlewares;

pub mod login;
pub mod invites;
pub mod promos;
pub mod users;
pub mod anon_users;
pub mod orders;
pub mod subscriptions;
pub mod files;
pub mod stats;
pub mod cloudflare;
pub mod early_access;
pub mod db;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .nest("/login", login::router())
        .nest("/invites", invites::router())
        .nest("/promos", promos::router())
        .nest("/users", users::router())
        .nest("/anon-users", anon_users::router())
        .nest("/orders", orders::router())
        .nest("/subscriptions", subscriptions::router())
        .nest("/files", files::router())
        .nest("/stats", stats::router())
        .nest("/cloudflare", cloudflare::router())
        .nest("/early-access", early_access::router())
        .nest("/db", db::router())
        // .route("/me", get(me::handle).layer(from_fn(middlewares::validate_admin_token)))
}

