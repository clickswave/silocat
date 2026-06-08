use axum::{
    body::Bytes,
    extract::{Query, State},
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, models, routes::respond};

/// POST /user/avatar  (protected)
/// Raw image bytes in the request body. We cap the size, normalize the image to
/// a fixed square JPEG (strips EXIF), store it in the DP bucket under the user's
/// id, and point users.profile_image at the avatar proxy URL (cache-busted).
pub async fn upload(
    State(state): State<crate::AppState>,
    Extension(user): Extension<models::UserTokenData>,
    body: Bytes,
) -> impl IntoResponse {
    if body.is_empty() {
        return respond(400, "No image data received", vec![], json!({}));
    }
    if body.len() > libs::image_dp::MAX_DP_BYTES {
        return respond(
            413,
            "Image too large",
            vec!["Display pictures must be 1 MB or smaller.".to_string()],
            json!({}),
        );
    }

    let normalized = match libs::image_dp::normalize(&body) {
        Ok(bytes) => bytes,
        Err(e) => return respond(400, "Invalid image", vec![e.to_string()], json!({})),
    };

    if let Err(e) = state
        .r2
        .put_object("dp", &user.id, normalized, "image/jpeg")
        .await
    {
        println!("[AVATAR] upload to R2 failed: {:?}", e);
        return respond(500, "Failed to store display picture", vec![], json!({}));
    }

    // Cache-busted proxy URL. Stored verbatim so the frontend can use it directly
    // as an <img src> (Google avatars are absolute http URLs; ours are relative).
    let version = chrono::Utc::now().timestamp_millis();
    let url = format!("/api/v1/user/avatar/{}?v={}", user.id, version);

    if let Err(e) = sqlx::query("UPDATE users SET profile_image = $1 WHERE id = $2")
        .bind(&url)
        .bind(&user.id)
        .execute(&state.pg_pool)
        .await
    {
        println!("[AVATAR] db update failed: {:?}", e);
        return respond(500, "Failed to save display picture", vec![], json!({}));
    }

    respond(
        200,
        "Display picture updated",
        vec![],
        json!({ "profile_image": url }),
    )
}

/// DELETE /user/avatar  (protected) - remove the uploaded avatar.
pub async fn remove(
    State(state): State<crate::AppState>,
    Extension(user): Extension<models::UserTokenData>,
) -> impl IntoResponse {
    // Best-effort object delete; clearing the DB field is what matters.
    let _ = state.r2.delete_object("dp", &user.id).await;

    if let Err(e) = sqlx::query("UPDATE users SET profile_image = NULL WHERE id = $1")
        .bind(&user.id)
        .execute(&state.pg_pool)
        .await
    {
        return respond(500, "Failed to remove display picture", vec![e.to_string()], json!({}));
    }

    respond(
        200,
        "Display picture removed",
        vec![],
        json!({ "profile_image": serde_json::Value::Null }),
    )
}

#[derive(Deserialize)]
pub struct AvatarQuery {
    pub user_id: String,
}

/// GET /user/avatar-url?user_id=...  (service-signed, called by the web_server
/// avatar proxy). Returns a short-lived presigned GET URL for the avatar object.
pub async fn presigned(
    State(state): State<crate::AppState>,
    Query(q): Query<AvatarQuery>,
) -> impl IntoResponse {
    match state.r2.presigned_get_url("dp", &q.user_id).await {
        Ok(url) => respond(200, "ok", vec![], json!({ "url": url })),
        Err(e) => respond(404, "Avatar not found", vec![e.to_string()], json!({})),
    }
}
