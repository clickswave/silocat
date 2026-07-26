use std::env;
use axum::extract::{Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use crate::routes::respond;

pub mod validate_token;
pub mod admin_secret;
pub mod resolve_identity;

/// Header name carrying the backend-to-backend secret. Configurable so the
/// published source does not fingerprint a deployment's wire protocol.
///
/// This is reconnaissance reduction, not a security control: anyone who observes
/// a single request learns the name. The secret VALUE is what protects the
/// surface, and nothing here should be relied on for confidentiality.
pub fn infra_header() -> String {
    env::var("INFRA_COMMUNICATION_SECRET_HEADER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "X-Authority-Sign".to_string())
}

/// Header name carrying the admin secret. Same reasoning as `infra_header`.
pub fn admin_header() -> String {
    env::var("ADMIN_COMMUNICATION_SECRET_HEADER")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "X-Admin-Secret".to_string())
}

/// Gate for every non-public route: only our own backends hold `INFRA_COMMUNICATION_SECRET`,
/// so a request without it never came from web_server or the admin panel. The
/// browser never sees this value; it is attached server-side during SSR.
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

    match request.headers().get(infra_header().as_str()) {
        None => unauthorized,
        Some(value) => {
            let header_val = match value.to_str() {
                Ok(v) => v,
                Err(_) => return unauthorized,
            };

            // Fail closed: an unset sign must never mean "allow everything".
            let env_sign = match env::var("INFRA_COMMUNICATION_SECRET") {
                Ok(v) if !v.trim().is_empty() => v,
                _ => return unauthorized,
            };

            if ct_eq(header_val.as_bytes(), env_sign.as_bytes()) {
                Ok(next.run(request).await)
            } else {
                unauthorized
            }
        }
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