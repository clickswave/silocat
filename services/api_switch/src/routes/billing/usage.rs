use crate::models::UserTokenData;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension};
use serde_json::json;

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(token): Extension<UserTokenData>,
) -> impl IntoResponse {
    let usage_res = sqlx::query!(
        "SELECT COALESCE(SUM(size), 0)::BIGINT as \"used_bytes!\" FROM files WHERE user_id = $1 AND deleted = FALSE",
        token.id
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

    match usage_res {
        Ok(row) => respond(
            200,
            "Usage fetched successfully",
            vec![],
            json!({
                "used_bytes": row.used_bytes,
            }),
        ),
        Err(e) => {
            println!("Failed to fetch usage: {}", e);
            respond(
                500,
                "Failed to fetch usage data",
                vec![],
                json!({}),
            )
        }
    }
}
