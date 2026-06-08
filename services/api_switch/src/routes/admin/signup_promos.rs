// Admin CRUD for signup storage-promo codes (see migration 0023).
// Runtime queries only (no query! macro) so the SQLX_OFFLINE prod build needs
// no regenerated cache for the new table.
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::routes::respond;

#[derive(Serialize, sqlx::FromRow)]
struct SignupPromo {
    code: String,
    description: String,
    bonus_bytes: i64,
    duration_days: Option<i32>,
    max_uses: Option<i32>,
    uses_count: i32,
    active: bool,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Deserialize)]
pub struct CreatePromo {
    pub code: String,
    #[serde(default)]
    pub description: String,
    pub bonus_bytes: i64,
    pub duration_days: Option<i32>,
    pub max_uses: Option<i32>,
    #[serde(default = "yes")]
    pub active: bool,
}
fn yes() -> bool {
    true
}

async fn list(State(state): State<crate::AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, SignupPromo>(
        "SELECT code, description, bonus_bytes, duration_days, max_uses, uses_count, active, created_at \
         FROM signup_promos ORDER BY created_at DESC",
    )
    .fetch_all(&state.pg_pool)
    .await;
    match rows {
        Ok(r) => respond(200, "Promos fetched", vec![], json!({ "promos": r })),
        Err(e) => respond(500, "Failed to list promos", vec![e.to_string()], json!(null)),
    }
}

async fn create(State(state): State<crate::AppState>, Json(p): Json<CreatePromo>) -> impl IntoResponse {
    let code = p.code.trim().to_string();
    if code.is_empty() {
        return respond(400, "Code required", vec!["Provide a promo code".to_string()], json!(null));
    }
    if p.bonus_bytes <= 0 {
        return respond(400, "Invalid bonus", vec!["bonus_bytes must be greater than 0".to_string()], json!(null));
    }
    let r = sqlx::query(
        "INSERT INTO signup_promos (code, description, bonus_bytes, duration_days, max_uses, active) \
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&code)
    .bind(&p.description)
    .bind(p.bonus_bytes)
    .bind(p.duration_days)
    .bind(p.max_uses)
    .bind(p.active)
    .execute(&state.pg_pool)
    .await;
    match r {
        Ok(_) => respond(201, "Promo created", vec![], json!({ "code": code })),
        Err(e) => {
            let unique = e.as_database_error().map(|d| d.is_unique_violation()).unwrap_or(false);
            let msg = if unique { "A promo with that code already exists".to_string() } else { e.to_string() };
            respond(if unique { 409 } else { 500 }, "Could not create promo", vec![msg], json!(null))
        }
    }
}

async fn remove(State(state): State<crate::AppState>, Path(code): Path<String>) -> impl IntoResponse {
    match sqlx::query("DELETE FROM signup_promos WHERE code = $1").bind(&code).execute(&state.pg_pool).await {
        Ok(_) => respond(200, "Promo deleted", vec![], json!({ "code": code })),
        Err(e) => respond(500, "Could not delete promo", vec![e.to_string()], json!(null)),
    }
}

async fn toggle(State(state): State<crate::AppState>, Path(code): Path<String>) -> impl IntoResponse {
    let r = sqlx::query_scalar::<_, bool>("UPDATE signup_promos SET active = NOT active WHERE code = $1 RETURNING active")
        .bind(&code)
        .fetch_optional(&state.pg_pool)
        .await;
    match r {
        Ok(Some(active)) => respond(200, "Promo toggled", vec![], json!({ "code": code, "active": active })),
        Ok(None) => respond(404, "No such promo", vec![format!("no promo {}", code)], json!(null)),
        Err(e) => respond(500, "Could not toggle promo", vec![e.to_string()], json!(null)),
    }
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{code}", delete(remove))
        .route("/{code}/toggle", post(toggle))
}
