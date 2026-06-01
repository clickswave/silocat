use axum::{extract::State, Json, response::IntoResponse, Router, routing::get};
use serde::Serialize;
use serde_json::json;
use crate::routes::respond;

#[derive(Serialize)]
pub struct BucketUsage {
    pub name: String,
    pub object_count: i64,
    pub total_size_bytes: i64,
}

pub async fn get_r2_usage(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let shadow_usage = state.r2.get_bucket_usage("shadow").await;
    let sanctum_usage = state.r2.get_bucket_usage("sanctum").await;

    let shadow = match shadow_usage {
        Ok((count, size)) => BucketUsage {
            name: "silo-cat-shadow".to_string(),
            object_count: count,
            total_size_bytes: size,
        },
        Err(e) => {
            eprintln!("Failed to get shadow usage: {:?}", e);
            BucketUsage { name: "silo-cat-shadow".to_string(), object_count: -1, total_size_bytes: -1 }
        }
    };

    let sanctum = match sanctum_usage {
        Ok((count, size)) => BucketUsage {
            name: "silo-cat-sanctum".to_string(),
            object_count: count,
            total_size_bytes: size,
        },
        Err(e) => {
            eprintln!("Failed to get sanctum usage: {:?}", e);
            BucketUsage { name: "silo-cat-sanctum".to_string(), object_count: -1, total_size_bytes: -1 }
        }
    };

    respond(
        200,
        "Cloudflare usage retrieved successfully",
        vec![],
        json!({
            "shadow": shadow,
            "sanctum": sanctum
        }),
    )
}

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/", get(get_r2_usage))
}
