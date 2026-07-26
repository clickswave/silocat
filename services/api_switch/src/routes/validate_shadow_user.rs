use axum::{extract::State, Json, response::IntoResponse};
use serde::Deserialize;
use serde_json::json;
use crate::libs;
use crate::routes::respond;

// Client may still send extra fields (ip/geo); they're ignored: the server is
// authoritative for IP + geolocation (client-reported values are spoofable).
#[derive(Deserialize, Debug)]
pub struct Payload {
    pub api_key: String,
    #[serde(default)]
    pub user_agent: String,
}

pub async fn handle(
    axum::extract::ConnectInfo(addr): axum::extract::ConnectInfo<std::net::SocketAddr>,
    State(axum_state): State<crate::AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Server-observed client IP + MaxMind geolocation.
    let ip = libs::geoip::client_ip(&headers, addr);

    // Anonymous IP ban: refuse the session so the client surfaces a "banned" toast.
    if let Some(ban) = libs::bans::ip_ban(&axum_state.pg_pool, &ip).await {
        return respond(
            403,
            "You are banned",
            vec![ban
                .reason
                .clone()
                .unwrap_or_else(|| "Your access has been banned.".to_string())],
            json!({ "banned": true, "reason": ban.reason, "until": ban.until }),
        );
    }

    let user_agent = if !payload.user_agent.trim().is_empty() {
        payload.user_agent.clone()
    } else {
        headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    };
    let geo_json = axum_state
        .geoip
        .as_ref()
        .and_then(|r| libs::geoip::lookup(r, &ip))
        .unwrap_or_else(|| json!({}));

    // The browser generates this key; store only its blind index.
    let api_key_index = match libs::apikey::blind_index(&payload.api_key) {
        Some(i) => i,
        None => return respond(500, "Server misconfigured", vec![], json!({})),
    };

    // Insert or update the anonymous user keyed by browser api_key.
    let insert_query = sqlx::query!(
        "INSERT INTO anonymous_users (api_key, ip_address, user_agent, geo_location, last_seen)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (api_key) DO UPDATE SET
         last_seen = NOW(),
         ip_address = EXCLUDED.ip_address,
         user_agent = EXCLUDED.user_agent,
         geo_location = EXCLUDED.geo_location",
        api_key_index,
        ip,
        user_agent,
        geo_json
    )
    .execute(&axum_state.pg_pool)
    .await;

    if let Err(e) = insert_query {
        println!("Error tracking anonymous user: {:?}", e);
        // Best-effort: never block the request on tracking.
    }

    respond(200, "Shadow user validated", vec![], json!({}))
}
