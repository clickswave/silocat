//! Admin gate.
//!
//! The admin surface is protected by the admin-capable caller's credential: a
//! high-entropy secret supplied in a header (name from `ADMIN_COMMUNICATION_HEADER`,
//! default `X-Admin`) and compared against `ADMIN_COMMUNICATION_SECRET`. It is a
//! distinct caller identity in the shared per-caller registry (see `super`), so
//! this file just delegates the match.
//!
//! This deliberately replaces the previous `admin_users` table + password login
//! + HMAC session tokens. That design required a seeded credential to exist in a
//! migration, which meant a real password hash sat in the repository: fatal for
//! a project going open source. There is now no admin account to seed, no hash
//! to leak, and nothing to crack, because the secret is random rather than
//! human-chosen.
//!
//! Two properties are kept from the old design and matter more than sessions:
//!
//! 1. It is a SEPARATE caller from `WEB_SERVER_COMMUNICATION_SECRET` (the
//!    frontend<->backend gate), so the web_server secret is never sufficient to
//!    reach admin.
//! 2. It FAILS CLOSED. With `ADMIN_COMMUNICATION_SECRET` unset or empty the admin
//!    caller is absent from the registry and the entire admin tree is unreachable.
//!
//! Tradeoff, stated plainly: a static secret carries no per-admin identity, no
//! expiry, and no way to revoke one operator without rotating for everyone. With
//! a single operator that costs nothing. If more people ever need admin access,
//! reintroduce per-admin sessions rather than passing this value around.

use crate::routes::respond;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// Middleware: require a valid ADMIN-capable caller credential on every admin
/// route. Delegates to the shared per-caller registry (constant-time compare,
/// fail-closed when no admin caller is configured).
pub async fn validate_admin_secret(request: Request, next: Next) -> Response {
    if super::admin_caller_authenticated(&request) {
        next.run(request).await
    } else {
        respond(
            401,
            "Unauthorized",
            vec!["Admin authentication required".to_string()],
            json!({}),
        )
        .into_response()
    }
}
