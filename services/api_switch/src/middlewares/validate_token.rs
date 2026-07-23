use crate::{models, routes::respond};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

pub async fn validate_token(
    State(state): State<crate::AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let unauthorized = respond(
        401,
        "Unauthorized",
        vec!["Invalid or missing API key".to_string()],
        json!({}),
    );

    let header_value = match request.headers().get("X-Api-Key") {
        Some(value) => value,
        None => return unauthorized.into_response(),
    };

    let api_key = match header_value.to_str() {
        Ok(value) => value,
        Err(_) => return unauthorized.into_response(),
    };

    let user = match sqlx::query_as::<_, models::User>(
        "SELECT * FROM users WHERE api_key = $1",
    )
    .bind(api_key)
        .fetch_optional(&state.pg_pool)
        .await
    {
        Ok(Some(user)) => user,
        _ => return unauthorized.into_response(),
    };

    // Banned users cannot use any protected API. Active ban = is_banned and the
    // (optional) expiry hasn't passed.
    let ban_active =
        user.is_banned && user.banned_until.map(|u| u > chrono::Utc::now()).unwrap_or(true);
    if ban_active {
        return respond(
            403,
            "Account banned",
            vec![user
                .ban_reason
                .clone()
                .unwrap_or_else(|| "Your account has been banned.".to_string())],
            json!({ "banned": true, "reason": user.ban_reason, "until": user.banned_until }),
        )
        .into_response();
    }

    let subscription = if let Some(sub_id) = &user.subscription_id {
        sqlx::query_as::<_, models::Subscription>("SELECT * FROM subscriptions WHERE id = $1 AND expires_on > NOW()")
            .bind(sub_id)
            .fetch_optional(&state.pg_pool)
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    let user_token_data = models::token_data(user, subscription);
    request.extensions_mut().insert(user_token_data);

    next.run(request).await
}
