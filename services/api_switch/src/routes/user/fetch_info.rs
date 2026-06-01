use crate::models::UserTokenData;
use crate::routes::respond;
use axum::{Extension, response::IntoResponse, Json};
use serde_json::json;

pub async fn handle(
    Extension(user): Extension<UserTokenData>,
) -> impl IntoResponse {
    respond(
        200,
        "User info fetched successfully",
        vec![],
        json!(user),
    )
}