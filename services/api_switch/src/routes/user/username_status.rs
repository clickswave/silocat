use axum::{extract::State, response::IntoResponse, Extension};
use serde_json::json;

use crate::{models, routes::respond};

/// GET /user/username-status  (protected)
/// How many username changes the user has left in the current rolling 30-day
/// window, and when the window resets. Drives the notice on the settings page.
pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<models::UserTokenData>,
) -> impl IntoResponse {
    const LIMIT: i32 = 2;

    let row = sqlx::query_as::<_, (i32, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT username_change_count, username_change_window_start FROM users WHERE id = $1",
    )
    .bind(&user.id)
    .fetch_optional(&state.pg_pool)
    .await;

    let (count, window_start) = match row {
        Ok(Some(r)) => r,
        Ok(None) => return respond(404, "User not found", vec![], json!({})),
        Err(e) => return respond(500, "Failed to read profile", vec![e.to_string()], json!({})),
    };

    let now = chrono::Utc::now();
    let window_active = window_start
        .map(|w| now.signed_duration_since(w).num_days() < 30)
        .unwrap_or(false);

    let (remaining, next_change_at) = if window_active {
        let remaining = (LIMIT - count).max(0);
        let next = window_start.unwrap() + chrono::Duration::days(30);
        (remaining, Some(next.to_rfc3339()))
    } else {
        (LIMIT, None)
    };

    respond(
        200,
        "ok",
        vec![],
        json!({
            "limit": LIMIT,
            "remaining": remaining,
            "next_change_at": next_change_at,
        }),
    )
}
