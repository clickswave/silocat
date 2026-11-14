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
    ///
    /// An object with a `user_id` belongs to that account and is matched on
    /// `user_id` alone. Its `owner_api_key` only records which key was current
    /// when it was made, and rotating the key must not leave that key able to
    /// reach it: once the old key is unknown to `users`, anyone still holding it
    /// can register it as an anonymous key, and a key match here would hand them
    /// the account's files and folders.
    ///
    /// Only an object with no account (an anonymous drop) is owned by the key
    /// that made it.
    pub fn owns(&self, user_id: &Option<String>, owner_api_key: &Option<String>) -> bool {
        match user_id {
            Some(owner) => self.user_id.as_deref() == Some(owner.as_str()),
            None => owner_api_key.as_deref() == Some(self.api_key.as_str()),
        }
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

    // Everything downstream (lookups, owns()) compares blind indexes, so the
    // raw key is converted here at the boundary and never stored or compared.
    let key = key.and_then(|k| crate::libs::apikey::blind_index(&k));

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
                    crate::libs::activity::touch(&state.pg_pool, &uid);
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

#[cfg(test)]
mod tests {
    use super::Caller;

    // owns() only ever sees blind indexes, so plain strings stand in for them.
    const OLD: &str = "index-of-the-rotated-out-key";
    const NEW: &str = "index-of-the-current-key";

    fn account(id: &str, key: &str) -> Caller {
        Caller {
            api_key: key.to_string(),
            user_id: Some(id.to_string()),
            email_verified: true,
        }
    }

    fn anonymous(key: &str) -> Caller {
        Caller {
            api_key: key.to_string(),
            user_id: None,
            email_verified: true,
        }
    }

    fn some(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn a_rotated_out_key_does_not_own_what_the_account_made_with_it() {
        // Everything the account created before rotating still carries the old
        // key's index. Rotation made that key unknown to `users`, so whoever still
        // holds it can only come back by registering it as an anonymous key.
        assert!(!anonymous(OLD).owns(&some("u1"), &some(OLD)));
    }

    #[test]
    fn the_account_keeps_its_objects_after_rotating() {
        assert!(account("u1", NEW).owns(&some("u1"), &some(OLD)));
        assert!(account("u1", NEW).owns(&some("u1"), &some(NEW)));
    }

    #[test]
    fn a_matching_key_does_not_reach_into_another_account() {
        assert!(!account("u2", OLD).owns(&some("u1"), &some(OLD)));
    }

    #[test]
    fn an_account_object_stamped_with_no_key_is_still_the_accounts() {
        // File-request uploads land in the requester's account with no key.
        assert!(account("u1", NEW).owns(&some("u1"), &None));
        assert!(!account("u2", NEW).owns(&some("u1"), &None));
        assert!(!anonymous(NEW).owns(&some("u1"), &None));
    }

    #[test]
    fn an_anonymous_drop_belongs_to_the_key_that_made_it() {
        assert!(anonymous(OLD).owns(&None, &some(OLD)));
        assert!(!anonymous(NEW).owns(&None, &some(OLD)));
        assert!(!anonymous(OLD).owns(&None, &None));
        // A row with no account and an account's key on it (left by uploads made
        // before identity was derived on the server) follows that key, which
        // rotation moves to the new one.
        assert!(account("u1", NEW).owns(&None, &some(NEW)));
        assert!(!account("u1", NEW).owns(&None, &some(OLD)));
    }
}
