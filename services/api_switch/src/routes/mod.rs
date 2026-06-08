// mod wordlist;
mod user;
mod auth;
mod file;
mod folder;
mod billing;
mod validate_shadow_user;
mod admin;

use axum::{Json, Router};
use axum::http::{StatusCode};
use axum::middleware::{from_fn};
use axum::response::{IntoResponse};
use axum::routing::post;
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use crate::middlewares;
use tower_http::cors::{CorsLayer, Any};

#[derive(Serialize, Deserialize)]
struct ResponseStruct {
    status: i32,
    message: String,
    errors: Vec<String>,
    data: Value,
}

pub fn respond(
    status: i32,
    message: &str,
    errors: Vec<String>,
    data: Value,
) -> impl IntoResponse {
    let status_code;

    match status {
        // success
        200 => status_code = StatusCode::OK,
        201 => status_code = StatusCode::CREATED,
        // client error
        400 => status_code = StatusCode::BAD_REQUEST,
        401 => status_code = StatusCode::UNAUTHORIZED,
        403 => status_code = StatusCode::FORBIDDEN,
        404 => status_code = StatusCode::NOT_FOUND,
        409 => status_code = StatusCode::CONFLICT,
        413 => status_code = StatusCode::PAYLOAD_TOO_LARGE,
        429 => status_code = StatusCode::TOO_MANY_REQUESTS,
        // server error
        500 => status_code = StatusCode::INTERNAL_SERVER_ERROR,
        _ => status_code = StatusCode::INTERNAL_SERVER_ERROR,
    }

    let message = message.to_string();
    (
        status_code,
        Json(ResponseStruct {
            status,
            message,
            errors,
            data,
        })
    )
}

pub async fn all(state: crate::AppState) -> Router<crate::AppState> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/validate-shadow-user", post(validate_shadow_user::handle))
        .nest("/user", user::router(state.clone()))
        .nest("/admin", admin::router())
        .nest("/auth", auth::router())
        .nest("/file", file::router())
        .nest("/folder", folder::router())
        .nest("/billing", billing::router())
        .layer(cors)
        .layer(from_fn(middlewares::authority_sign_check))
}