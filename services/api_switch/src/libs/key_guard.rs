//! Binding a write to the key that authenticated it.
//!
//! A request is authenticated once, by its key, at the start. Its writes run
//! later. If the account's key is replaced in between (a rotation, or the owner
//! of the address claiming an account someone else registered), a write that
//! hands out a credential or publishes data would still land for whoever held
//! the old key: a new password, a fresh key, a share link. Handlers that do
//! that lock the account row through `still_current` first, in the transaction
//! that writes, and refuse when it says no.
//!
//! The lock is what makes the check hold. A key change that commits first is
//! seen here, since a locking read waits for it and then rereads the row. One
//! that starts later waits for this transaction, and runs after the write it
//! is meant to undo.

use sqlx::PgConnection;

/// Lock the account row while it still carries the key with blind index
/// `key_index`. `Ok(false)` when it no longer does (or the account is gone).
pub async fn still_current(
    conn: &mut PgConnection,
    user_id: &str,
    key_index: &str,
) -> Result<bool, sqlx::Error> {
    let held: Option<i32> =
        sqlx::query_scalar("SELECT 1 FROM users WHERE id = $1 AND api_key = $2 FOR UPDATE")
            .bind(user_id)
            .bind(key_index)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(held.is_some())
}
