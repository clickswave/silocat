use crate::routes::respond;
use axum::{extract::{State, Query}, response::IntoResponse, Json};
use serde::{Deserialize};
use serde_json::json;

#[derive(Deserialize)]
pub struct UsageParams {
    pub user_id: String,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Query(params): Query<UsageParams>,
) -> impl IntoResponse {
    let usage_res = sqlx::query!(
        "SELECT COALESCE(SUM(size), 0)::BIGINT as \"used_bytes!\" FROM files WHERE user_id = $1 AND deleted = FALSE",
        params.user_id
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
                // We could also return limit here if we query user, but limit is already in user token data usually
            }),
        ),
        Err(e) => {
            println!("Failed to fetch usage: {}", e);
            respond(
                500,
                "Failed to fetch usage data",
                vec![e.to_string()],
                json!({}),
            )
        }
    }
}
