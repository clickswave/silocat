//! Server-side IP observation + GeoLite2-City geolocation (ported from crossfyre).
//!
//! SiloCat sits behind Cloudflare -> nginx -> api_switch, so the real client IP
//! is in `CF-Connecting-IP` (Cloudflare) or `X-Forwarded-For` / `X-Real-IP`
//! (nginx). Direct dev hits land as the socket peer (a private docker IP), which
//! the lookup simply can't resolve (no public geo) and returns None.
//!
//! Geolocation uses a local MaxMind GeoLite2-City database (no network calls),
//! loaded once at startup and shared via AppState. It no-ops gracefully when the
//! db file is absent, so nothing breaks if it isn't shipped to a box.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use axum::http::HeaderMap;

pub type GeoReader = Arc<maxminddb::Reader<Vec<u8>>>;

/// Open the GeoLite2-City db once at startup. Path from `GEOLITE2_DB_PATH`
/// (defaults to `GeoLite2-City.mmdb` in the working dir). Returns None (with a
/// warning) if absent, so geolocation simply no-ops until the db is present.
pub fn load() -> Option<GeoReader> {
    let path = std::env::var("GEOLITE2_DB_PATH").unwrap_or_else(|_| "GeoLite2-City.mmdb".to_string());
    match maxminddb::Reader::open_readfile(&path) {
        Ok(reader) => {
            println!("[geoip] loaded GeoLite2 database from {}", path);
            Some(Arc::new(reader))
        }
        Err(e) => {
            println!("[geoip] could not open {} ({}). Geolocation disabled.", path, e);
            None
        }
    }
}

/// Resolve the real client IP, preferring proxy headers and falling back to the
/// socket peer. CF-Connecting-IP (Cloudflare) -> X-Forwarded-For (first hop) ->
/// X-Real-IP -> ConnectInfo peer.
pub fn client_ip(headers: &HeaderMap, peer: SocketAddr) -> String {
    if let Some(cf) = headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()) {
        let ip = cf.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }
    if let Some(xr) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let ip = xr.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    peer.ip().to_string()
}

/// Full geolocation JSON for an IP (stored on anonymous_users.geo_location):
/// country code/name, city, lat/lng. None for unresolvable (private/loopback) IPs.
pub fn lookup(reader: &GeoReader, ip_str: &str) -> Option<serde_json::Value> {
    let ip: IpAddr = ip_str.parse().ok()?;
    let city: maxminddb::geoip2::City = reader.lookup(ip).ok()?;

    let lat = city.location.as_ref().and_then(|l| l.latitude);
    let lng = city.location.as_ref().and_then(|l| l.longitude);
    let country = city.country.as_ref().and_then(|c| c.iso_code).map(|s| s.to_string());
    let country_name = city
        .country
        .as_ref()
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());
    let region = city
        .subdivisions
        .as_ref()
        .and_then(|s| s.first())
        .and_then(|s| s.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());
    let city_name = city
        .city
        .as_ref()
        .and_then(|c| c.names.as_ref())
        .and_then(|n| n.get("en"))
        .map(|s| s.to_string());

    // Nothing useful resolved -> treat as unlocatable.
    if country.is_none() && lat.is_none() && lng.is_none() {
        return None;
    }

    Some(serde_json::json!({
        "country": country,
        "country_name": country_name,
        "region": region,
        "city": city_name,
        "lat": lat,
        "lng": lng,
    }))
}

/// Just the ISO country code for an IP (stored on users.country, which the
/// CountrySelect keys by 2-letter code). None for unresolvable IPs.
pub fn country_code(reader: &GeoReader, ip_str: &str) -> Option<String> {
    let ip: IpAddr = ip_str.parse().ok()?;
    let city: maxminddb::geoip2::City = reader.lookup(ip).ok()?;
    city.country.as_ref().and_then(|c| c.iso_code).map(|s| s.to_string())
}
