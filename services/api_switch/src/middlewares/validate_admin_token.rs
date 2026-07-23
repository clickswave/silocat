use crate::routes::respond;
use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Identity attached to the request once an admin token is verified.
#[derive(Clone, Debug)]
pub struct AdminIdentity {
    pub id: String,
}

/// The signing secret for admin session tokens. This is deliberately SEPARATE
/// from `AUTHORITY_SIGN` (the shared frontend<->backend gate): knowing the shared
/// sign must NOT be enough to forge an admin session. If it is unset/empty the
/// admin surface is **disabled** (fail closed) rather than falling back to a
/// derivable key.
fn admin_secret() -> Option<String> {
    std::env::var("ADMIN_TOKEN_SECRET")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn sign(key: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(key.as_bytes()).expect("HMAC accepts any key length");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Mint a stateless admin session token: `{admin_id}.{exp}.{sig}` where
/// `sig = hex(HMAC-SHA256(ADMIN_TOKEN_SECRET, "{admin_id}.{exp}"))`.
/// Returns `None` if the admin surface is not configured (secret unset).
pub fn mint(admin_id: &str, ttl_secs: i64) -> Option<String> {
    let secret = admin_secret()?;
    let exp = chrono::Utc::now().timestamp() + ttl_secs;
    let payload = format!("{}.{}", admin_id, exp);
    let sig = sign(&secret, &payload);
    Some(format!("{}.{}", payload, sig))
}

/// Verify a token; returns the admin id on success. Constant-time signature
/// check (via `Mac::verify_slice`), rejects on expiry or if admin is disabled.
fn verify(token: &str) -> Option<String> {
    let secret = admin_secret()?;
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return None;
    }
    let (admin_id, exp_str, sig_hex) = (parts[0], parts[1], parts[2]);
    let payload = format!("{}.{}", admin_id, exp_str);

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    let sig = hex::decode(sig_hex).ok()?;
    mac.verify_slice(&sig).ok()?; // constant-time

    let exp: i64 = exp_str.parse().ok()?;
    if chrono::Utc::now().timestamp() > exp {
        return None;
    }
    Some(admin_id.to_string())
}

/// Middleware: require a valid `X-Admin-Token` on every admin route. Fails closed
/// when `ADMIN_TOKEN_SECRET` is unset — the admin API is unreachable until it is
/// configured, so the shared `AUTHORITY_SIGN` alone can never reach admin.
pub async fn validate_admin_token(mut request: Request, next: Next) -> Response {
    let unauthorized = respond(
        401,
        "Unauthorized",
        vec!["Admin authentication required".to_string()],
        json!({}),
    );

    let admin_id = match request
        .headers()
        .get("X-Admin-Token")
        .and_then(|v| v.to_str().ok())
        .and_then(verify)
    {
        Some(id) => id,
        None => return unauthorized.into_response(),
    };

    request.extensions_mut().insert(AdminIdentity { id: admin_id });
    next.run(request).await
}
