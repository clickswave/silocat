use axum::{routing::get, Json, Router, extract::State};
use serde_json::json;
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", get(list_requests))
}

pub async fn list_requests(
    State(state): State<crate::AppState>,
) -> impl axum::response::IntoResponse {
    let requests = sqlx::query!(
        "SELECT id, email, status, created_on FROM early_access_requests ORDER BY created_on DESC"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match requests {
        Ok(requests) => {
            let requests_json: Vec<_> = requests
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id.to_string(),
                        "email": r.email,
                        "status": r.status,
                        "created_on": r.created_on
                    })
                })
                .collect();
            respond(200, "Requests fetched", vec![], json!(requests_json))
        }
        Err(e) => {
            println!("Error fetching early access requests: {:?}", e);
            respond(500, "Failed to fetch requests", vec![], json!({}))
        }
    }
}
