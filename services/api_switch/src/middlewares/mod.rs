use std::env;
use axum::extract::{Request};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use crate::routes::respond;

pub mod validate_token;

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

    match request.headers().get("X-Authority-Sign") {
        None => unauthorized,
        Some(value) => {
            let header_val = match value.to_str() {
                Ok(v) => v,
                Err(_) => return unauthorized,
            };

            let env_sign = match env::var("AUTHORITY_SIGN") {
                Ok(v) => v,
                Err(_) => return unauthorized, 
            };
            
            if header_val == env_sign {
                Ok(next.run(request).await)
            } else {
                unauthorized
            }
        }
    }
}