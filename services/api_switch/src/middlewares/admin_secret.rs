//! Admin gate.
//!
//! The admin surface is protected by a single high-entropy secret supplied in
//! a header (name from `ADMIN_COMMUNICATION_SECRET_HEADER`, default `X-Admin-Secret`)
//! and compared against `ADMIN_COMMUNICATION_SECRET` from the environment.
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
//! 1. It is SEPARATE from `INFRA_COMMUNICATION_SECRET` (the shared frontend<->backend gate),
//!    so knowing the shared sign is never sufficient to reach admin.
//! 2. It FAILS CLOSED. With `ADMIN_COMMUNICATION_SECRET` unset or empty the entire admin tree
//!    is unreachable, rather than falling back to a derivable or default value.
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

/// The configured admin secret, or `None` when the surface is disabled.
fn admin_secret() -> Option<String> {
    std::env::var("ADMIN_COMMUNICATION_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Constant-time comparison so a timing side channel cannot leak the secret one
/// byte at a time.
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Middleware: require a valid `X-Admin-Secret` on every admin route.
pub async fn validate_admin_secret(request: Request, next: Next) -> Response {
    let unauthorized = respond(
        401,
        "Unauthorized",
        vec!["Admin authentication required".to_string()],
        json!({}),
    );

    // No secret configured => the admin surface does not exist.
    let expected = match admin_secret() {
        Some(s) => s,
        None => return unauthorized.into_response(),
    };

    let provided = request
        .headers()
        .get(super::admin_header().as_str())
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !ct_eq(provided.as_bytes(), expected.as_bytes()) {
        return unauthorized.into_response();
    }

    next.run(request).await
}
