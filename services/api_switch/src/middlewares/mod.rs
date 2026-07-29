use std::env;
use axum::extract::{Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use crate::routes::respond;

pub mod validate_token;
pub mod admin_secret;
pub mod resolve_identity;

// ---- per-caller internal auth -----------------------------------------------
// Identical model to crossfyre's api_switch: each backend authenticates with its
// OWN header + secret (per-service credentials), so a call is attributable to a
// specific service and one service's secret rotates without touching the others.
// Header names are configurable so the published source does not fingerprint the
// wire protocol; the VALUE is the control, not the name.

struct Caller {
    header: String,
    secret: String,
    admin: bool,
}

fn header_name(var: &str, default: &str) -> String {
    env::var(var)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn nonempty_env(var: &str) -> Option<String> {
    env::var(var).ok().map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

/// Known internal callers, built from the environment. A caller is present only
/// when its secret is set; an empty registry fails the whole surface closed.
fn callers() -> Vec<Caller> {
    let mut v = Vec::new();
    if let Some(secret) = nonempty_env("WEB_SERVER_COMMUNICATION_SECRET") {
        v.push(Caller {
            header: header_name("WEB_SERVER_COMMUNICATION_HEADER", "X-Web-Server"),
            secret,
            admin: false,
        });
    }
    if let Some(secret) = nonempty_env("ADMIN_COMMUNICATION_SECRET") {
        v.push(Caller {
            header: header_name("ADMIN_COMMUNICATION_HEADER", "X-Admin"),
            secret,
            admin: true,
        });
    }
    v
}

fn caller_authenticated(request: &Request, want: impl Fn(&Caller) -> bool) -> bool {
    for c in callers() {
        if !want(&c) {
            continue;
        }
        if let Some(h) = request.headers().get(c.header.as_str()).and_then(|v| v.to_str().ok()) {
            if ct_eq(h.as_bytes(), c.secret.as_bytes()) {
                return true;
            }
        }
    }
    false
}

/// A valid credential for ANY known caller (used by the infra gate).
pub(crate) fn any_caller_authenticated(request: &Request) -> bool {
    caller_authenticated(request, |_| true)
}

/// A valid credential for an ADMIN-capable caller (used by the admin gate).
pub(crate) fn admin_caller_authenticated(request: &Request) -> bool {
    caller_authenticated(request, |c| c.admin)
}

/// Gate for every non-public route: only our own backends hold a caller secret,
/// so a request without a valid one never came from web_server or the admin
/// panel. The browser never sees these values; they are attached server-side.
pub async fn authority_sign_check(
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let unauthorized = Err(respond(
        401,
        "Unauthorized Origin",
        vec![],
        json!({}),
    ));

    if any_caller_authenticated(&request) {
        Ok(next.run(request).await)
    } else {
        unauthorized
    }
}

/// Constant-time comparison, so a timing side channel cannot leak the sign one
/// byte at a time.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}