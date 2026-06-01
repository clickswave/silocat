use axum::{extract::State, Json, response::IntoResponse};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json};
use crate::{libs, models};
use crate::routes::respond;

#[derive(Deserialize, Serialize, Debug)]
struct GeoInfo {
    country: String,
    region: String,
    city: String,
    latitude: String,
    longitude: String,
    asn: String,
    isp: String,
}

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub api_key: String,
    pub user_agent: String,
    pub ip: String,
    pub geo: GeoInfo,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {

    println!("HIT VALIDATE SHADOW USER");

    let geo_json = serde_json::to_value(&payload.geo).unwrap_or(json!({}));

    // Insert or Update anonymous user
    let insert_query = sqlx::query!(
        "INSERT INTO anonymous_users (api_key, ip_address, user_agent, geo_location, last_seen)
         VALUES ($1, $2, $3, $4, NOW())
         ON CONFLICT (api_key) DO UPDATE SET
         last_seen = NOW(),
         ip_address = EXCLUDED.ip_address,
         user_agent = EXCLUDED.user_agent,
         geo_location = EXCLUDED.geo_location",
        payload.api_key,
        payload.ip,
        payload.user_agent,
        geo_json
    )
    .execute(&axum_state.pg_pool)
    .await;

    if let Err(e) = insert_query {
        println!("Error tracking anonymous user: {:?}", e);
        // We don't block the request, just log error
    }

    respond(
        200,
        "Shadow user validated",
        vec![],
        json!({}),
    )
}
