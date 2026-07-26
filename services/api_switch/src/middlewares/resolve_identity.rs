use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

/// The authenticated caller behind an `X-Api-Key`. The file/folder routes serve
/// both registered users ("sanctum") and anonymous ("shadow") sessions, so a
/// caller is either a real user or a known shadow key. Resolved server-side from
/// the key: never from a client-supplied `user_id`/`owner_api_key` body field.
#[derive(Clone, Debug)]
pub struct Caller {
    /// The presented key: a user's `api_key` or a shadow anonymous `api_key`.
    pub api_key: String,
    /// `Some(user_id)` for a registered user; `None` for a shadow session.
    pub user_id: Option<String>,
    /// Whether the account's email is verified. Always true for shadow sessions
    /// (anonymous, no email); used to gate actions like uploads.
    pub email_verified: bool,
}

impl Caller {
    /// Does this caller own an object with the given `user_id` / `owner_api_key`?
    /// True when the object's stored owner key matches the caller's key, or the
    /// object belongs to the caller's account.
    pub fn owns(&self, user_id: &Option<String>, owner_api_key: &Option<String>) -> bool {
        if owner_api_key.as_deref() == Some(self.api_key.as_str()) {
            return true;
        }
        matches!((user_id, &self.user_id), (Some(a), Some(b)) if a == b)
    }
}

/// Non-rejecting identity resolver for the file/folder routers. Attaches an
/// `Option<Caller>` to every request: `Some` when the `X-Api-Key` resolves to a
/// user or a shadow session, `None` otherwise. Handlers decide what to require: /// reads allow public objects with no caller; writes demand ownership.
pub async fn resolve_identity(
    State(state): State<crate::AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let key = request
        .headers()
        .get("X-Api-Key")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .filter(|k| !k.is_empty());

    let caller: Option<Caller> = match key {
        Some(k) => {
            // Registered user?
            let user = sqlx::query_as::<_, (String, bool)>(
                "SELECT id, email_verified FROM users WHERE api_key = $1",
            )
            .bind(&k)
            .fetch_optional(&state.pg_pool)
            .await
            .ok()
            .flatten();
            match user {
                Some((uid, email_verified)) => {
                    Some(Caller { api_key: k, user_id: Some(uid), email_verified })
                }
                None => {
                    // Known shadow (anonymous) session?
                    let is_shadow = sqlx::query_scalar::<_, String>(
                        "SELECT api_key FROM anonymous_users WHERE api_key = $1",
                    )
                    .bind(&k)
                    .fetch_optional(&state.pg_pool)
                    .await
                    .ok()
                    .flatten()
                    .is_some();
                    if is_shadow {
                        Some(Caller { api_key: k, user_id: None, email_verified: true })
                    } else {
                        None
                    }
                }
            }
        }
        None => None,
    };

    request.extensions_mut().insert(caller);
    next.run(request).await
}
