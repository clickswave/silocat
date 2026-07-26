//! POST /user/rotate-api-key: issue a fresh API key for the calling user.
//!
//! There is one key per account and it is the credential for the whole
//! programmatic surface, so rotation is immediate and total: every integration
//! using the old key stops working the moment this returns. The UI says so
//! before asking for confirmation.
//!
//! The new key is returned once, in the response. We never email it and there is
//! no "show me the old one" path.

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
) -> impl IntoResponse {
    let user_id = token_user.id;

    let minted = match crate::libs::apikey::mint() {
        Some(k) => k,
        None => return respond(500, "Could not rotate API key", vec![], json!({})),
    };
    let new_key = minted.raw;

    let result = sqlx::query(
        "UPDATE users SET api_key = $1, api_key_enc = $2, api_key_migrated = TRUE WHERE id = $3",
    )
    .bind(&minted.blind_index)
    .bind(&minted.encrypted)
    .bind(&user_id)
    .execute(&axum_state.pg_pool)
    .await;

    match result {
        Ok(r) if r.rows_affected() == 1 => respond(
            200,
            "API key rotated",
            vec![],
            json!({ "api_key": new_key }),
        ),
        Ok(_) => respond(404, "User not found", vec![], json!({})),
        Err(e) => {
            eprintln!("[rotate_api_key] {:?}", e);
            respond(500, "Could not rotate API key", vec![], json!({}))
        }
    }
}
