use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, delete},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::libs::rng;
use crate::routes::respond;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_ip_bans).post(create_ip_ban))
        .route("/{id}", delete(delete_ip_ban))
}

async fn list_ip_bans(State(state): State<crate::AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (String, String, Option<String>, Option<chrono::DateTime<chrono::Utc>>, chrono::DateTime<chrono::Utc>)>(
        "SELECT id, ip, reason, banned_until, created_at FROM ip_bans ORDER BY created_at DESC LIMIT 500",
    )
    .fetch_all(&state.pg_pool)
    .await;

    match rows {
        Ok(rows) => {
            let bans: Vec<_> = rows
                .into_iter()
                .map(|(id, ip, reason, banned_until, created_at)| {
                    let active = banned_until.map(|u| u > chrono::Utc::now()).unwrap_or(true);
                    json!({
                        "id": id,
                        "ip": ip,
                        "reason": reason,
                        "banned_until": banned_until,
                        "created_at": created_at,
                        "permanent": banned_until.is_none(),
                        "active": active,
                    })
                })
                .collect();
            respond(200, "IP bans fetched", vec![], json!({ "bans": bans }))
        }
        Err(e) => respond(500, "Failed to fetch IP bans", vec![e.to_string()], json!({})),
    }
}

#[derive(Deserialize)]
struct CreateIpBan {
    ip: String,
    reason: Option<String>,
    // None / 0 / negative => permanent.
    days: Option<i64>,
}

async fn create_ip_ban(
    State(state): State<crate::AppState>,
    Json(body): Json<CreateIpBan>,
) -> impl IntoResponse {
    let ip = body.ip.trim().to_string();
    if ip.is_empty() {
        return respond(400, "IP is required", vec![], json!({}));
    }
    let id = rng::uuid();
    let until = match body.days {
        Some(d) if d > 0 => Some(chrono::Utc::now() + chrono::Duration::days(d)),
        _ => None, // permanent
    };

    let res = sqlx::query(
        "INSERT INTO ip_bans (id, ip, reason, banned_until) VALUES ($1, $2, $3, $4)",
    )
    .bind(&id)
    .bind(&ip)
    .bind(&body.reason)
    .bind(until)
    .execute(&state.pg_pool)
    .await;

    match res {
        Ok(_) => respond(200, "IP banned", vec![], json!({ "id": id })),
        Err(e) => respond(500, "Failed to create IP ban", vec![e.to_string()], json!({})),
    }
}

async fn delete_ip_ban(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let res = sqlx::query("DELETE FROM ip_bans WHERE id = $1")
        .bind(&id)
        .execute(&state.pg_pool)
        .await;
    match res {
        Ok(_) => respond(200, "IP ban removed", vec![], json!({})),
        Err(e) => respond(500, "Failed to remove IP ban", vec![e.to_string()], json!({})),
    }
}
