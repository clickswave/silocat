use axum::Router;
use axum::middleware::from_fn;
use crate::middlewares;

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
// `db` (arbitrary read-only SQL browser) is intentionally NOT mounted: it is a
// full data-exfiltration primitive and has no place on a public product. The
// module is kept for local debugging only.
#[allow(dead_code)]
pub mod db;
pub mod signup_promos;
pub mod ip_bans;
pub mod support_tickets;

pub fn router() -> Router<crate::AppState> {
    // Every admin route requires the admin caller's credential (X-Admin, checked
    // against ADMIN_COMMUNICATION_SECRET). Fails closed: with the secret unset the
    // whole tree is unreachable, and the web_server caller secret is never
    // sufficient to reach admin. There is no login route because there is no admin
    // account, only the secret.
    Router::new()
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
        .nest("/signup-promos", signup_promos::router())
        .nest("/ip-bans", ip_bans::router())
        .nest("/support-tickets", support_tickets::router())
        .layer(from_fn(middlewares::admin_secret::validate_admin_secret))
}

