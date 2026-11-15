//! POST /user/rotate-api-key: issue a fresh API key for the calling user.
//!
//! There is one key per account and it is the credential for the whole
//! programmatic surface, so rotation is immediate and total: every integration
//! using the old key stops working the moment this returns. The UI says so
//! before asking for confirmation.
//!
//! The new key is returned once, in the response. We never email it and there is
//! no "show me the old one" path.
//!
//! Files and folders record the key that made them in `owner_api_key`. For a row
//! with no `user_id` that column is the only owner it has, and older uploads
//! left account keys on such rows. If those rows kept the old key, whoever still
//! holds it could register it as an anonymous key and own them. So they move to
//! the new key in the same transaction as the user row, the way apikey_backfill
//! moved them to the blind index.

use crate::middlewares::validate_token::PresentedKey;
use crate::models::UserTokenData;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension};
use serde_json::json;
use uuid::Uuid;

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    // Same extension the other /user routes use: this router is wrapped in
    // `validate_token`, so reaching this handler already proves an account.
    Extension(token_user): Extension<UserTokenData>,
    Extension(key): Extension<PresentedKey>,
) -> impl IntoResponse {
    let user_id = token_user.id;

    let minted = match crate::libs::apikey::mint() {
        Some(k) => k,
        None => return respond(500, "Could not rotate API key", vec![], json!({})),
    };
    let new_key = minted.raw;

    let result = rotate(
        &axum_state.pg_pool,
        &user_id,
        &key.0,
        &minted.blind_index,
        &minted.encrypted,
    )
    .await;

    match result {
        Ok(true) => respond(
            200,
            "API key rotated",
            vec![],
            json!({ "api_key": new_key }),
        ),
        // The key this request came in with is no longer the account's.
        Ok(false) => respond(
            401,
            "Unauthorized",
            vec!["Invalid or missing API key".to_string()],
            json!({}),
        ),
        Err(e) => {
            eprintln!("[rotate_api_key] {:?}", e);
            respond(500, "Could not rotate API key", vec![], json!({}))
        }
    }
}

/// Replace the account's key and move the objects stamped with the old one.
/// `Ok(false)` when the account no longer has the key `presented` (the index
/// of the key the request authenticated with), or there is no such user. A
/// rotation let in on a key that has since been replaced would otherwise hand
/// whoever held that key a fresh one.
async fn rotate(
    pool: &sqlx::PgPool,
    user_id: &str,
    presented: &str,
    new_index: &str,
    new_encrypted: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if !rotate_in(&mut tx, user_id, presented, new_index, new_encrypted).await? {
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}

pub(super) async fn rotate_in(
    conn: &mut sqlx::PgConnection,
    user_id: &str,
    presented: &str,
    new_index: &str,
    new_encrypted: &str,
) -> Result<bool, sqlx::Error> {
    if !crate::libs::key_guard::still_current(conn, user_id, presented).await? {
        return Ok(false);
    }
    replace_key(conn, user_id, new_index, new_encrypted).await
}

/// The body of a rotation, for callers that already hold a transaction (an
/// account claimed by the owner of its address rotates the key the same way).
/// `Ok(false)` when there is no such user.
pub(super) async fn replace_key(
    conn: &mut sqlx::PgConnection,
    user_id: &str,
    new_index: &str,
    new_encrypted: &str,
) -> Result<bool, sqlx::Error> {
    // Locked, so two rotations at once cannot both read the same old key and
    // leave the objects on the key the first one issued.
    let old: Option<Option<String>> =
        sqlx::query_scalar("SELECT api_key FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
    let Some(old) = old else { return Ok(false) };

    if let Some(old) = old.as_deref() {
        sqlx::query("UPDATE files SET owner_api_key = $1 WHERE owner_api_key = $2")
            .bind(new_index)
            .bind(old)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE folders SET owner_api_key = $1 WHERE owner_api_key = $2")
            .bind(new_index)
            .bind(old)
            .execute(&mut *conn)
            .await?;
    }

    sqlx::query(
        "UPDATE users SET api_key = $1, api_key_enc = $2, api_key_migrated = TRUE WHERE id = $3",
    )
    .bind(new_index)
    .bind(new_encrypted)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;

    Ok(true)
}
