use axum::extract::{ConnectInfo, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;
use std::net::SocketAddr;

use crate::routes::respond;

#[derive(Deserialize)]
pub struct ReportInput {
    /// The share token (or link) being reported.
    pub share_token: Option<String>,
    pub reason: String,
    pub details: Option<String>,
}

/// POST /report (public): anyone can report an abusive share link. Reports are
/// stored for admin review + takedown; we do not auto-disable (that would let
/// anyone censor a link), the admin actions it.
pub async fn handle(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<crate::AppState>,
    headers: HeaderMap,
    Json(payload): Json<ReportInput>,
) -> impl IntoResponse {
    let ip = crate::libs::geoip::client_ip(&headers, addr);
    if !state.rate_limiter.check(&format!("report:{}", ip), 20, std::time::Duration::from_secs(3600)) {
        return respond(429, "Too Many Requests", vec!["Please try again later.".to_string()], json!({}));
    }

    if payload.reason.trim().is_empty() {
        return respond(400, "A reason is required", vec![], json!({}));
    }

    // Bound the free-text so a report can't be used to store huge blobs.
    let reason: String = payload.reason.trim().chars().take(2000).collect();
    let details: Option<String> = payload
        .details
        .map(|d| d.trim().chars().take(5000).collect::<String>())
        .filter(|d| !d.is_empty());
    let share_token: Option<String> =
        payload.share_token.map(|t| t.chars().take(256).collect::<String>());

    let res = sqlx::query(
        "INSERT INTO abuse_reports (share_token, reason, details, reporter_ip) VALUES ($1, $2, $3, $4)",
    )
    .bind(share_token)
    .bind(reason)
    .bind(details)
    .bind(ip)
    .execute(&state.pg_pool)
    .await;

    match res {
        Ok(_) => respond(
            200,
            "Report received",
            vec!["Thank you. Our team will review this report.".to_string()],
            json!({}),
        ),
        Err(_e) => respond(500, "Could not submit report", vec![], json!({})),
    }
}
