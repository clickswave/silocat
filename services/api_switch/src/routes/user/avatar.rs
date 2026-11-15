use axum::{
    body::Bytes,
    extract::{Query, State},
    response::IntoResponse,
    Extension,
};
use serde::Deserialize;
use serde_json::json;

use crate::{libs, models, routes::respond};

/// Avatar uploads one account may make per window.
const AVATAR_UPLOADS_PER_WINDOW: u32 = 10;
const AVATAR_UPLOAD_WINDOW: std::time::Duration = std::time::Duration::from_secs(600);

/// Avatars being decoded at once. Each decode holds a blocking thread and a
/// core for up to a few hundred milliseconds; later uploads wait their turn
/// without holding a runtime worker.
static DECODES: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
const DECODE_WAIT: std::time::Duration = std::time::Duration::from_secs(10);

/// POST /user/avatar  (protected)
/// Raw image bytes in the request body. We cap the size, normalize the image to
/// a fixed square JPEG (strips EXIF), store it in the DP bucket under the user's
/// id, and point users.profile_image at the avatar proxy URL (cache-busted).
pub async fn upload(
    State(state): State<crate::AppState>,
    Extension(user): Extension<models::UserTokenData>,
    body: Bytes,
) -> impl IntoResponse {
    if !state.rate_limiter.check(
        &format!("avatar:{}", user.id), AVATAR_UPLOADS_PER_WINDOW, AVATAR_UPLOAD_WINDOW)
    {
        return respond(429, "Too Many Requests",
            vec!["Too many display picture uploads. Please try again later.".to_string()], json!({}));
    }
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

    // Decoding and resizing is CPU-bound, so it runs on the blocking pool, never
    // on a runtime worker that every other route needs. The permit moves into
    // the closure so it is held until the decode ends, even if the client
    // goes away first.
    let permit = match tokio::time::timeout(DECODE_WAIT, DECODES.acquire()).await {
        Ok(Ok(permit)) => permit,
        _ => {
            return respond(503, "Server busy",
                vec!["Please try again in a moment.".to_string()], json!({}));
        }
    };
    let decoded = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        libs::image_dp::normalize(&body)
    })
    .await;
    let normalized = match decoded {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(e)) if e.is::<libs::image_dp::TooLarge>() => {
            return respond(
                413,
                "Image too large",
                vec!["Display pictures must be 16 megapixels or smaller.".to_string()],
                json!({}),
            );
        }
        Ok(Err(_e)) => return respond(400, "Invalid image", vec![], json!({})),
        Err(e) => {
            println!("[AVATAR] normalize task failed: {:?}", e);
            return respond(500, "Failed to process display picture", vec![], json!({}));
        }
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

    if let Err(_e) = sqlx::query("UPDATE users SET profile_image = NULL WHERE id = $1")
        .bind(&user.id)
        .execute(&state.pg_pool)
        .await
    {
        return respond(500, "Failed to remove display picture", vec![], json!({}));
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
        Err(_e) => respond(404, "Avatar not found", vec![], json!({})),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    fn user(id: &str) -> models::UserTokenData {
        models::UserTokenData {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            profile_image: None,
            email_verified: true,
            password_set: true,
            username: id.to_string(),
            api_key: String::new(),
            account_type: "personal".to_string(),
            subscription: None,
            default_storage_bytes: 0,
            country: None,
            bio: None,
        }
    }

    async fn status(state: &crate::AppState, id: &str, body: Vec<u8>) -> u16 {
        upload(State(state.clone()), Extension(user(id)), Bytes::from(body))
            .await
            .into_response()
            .status()
            .as_u16()
    }

    /// The test runtime has one thread, like a busy server whose workers are
    /// all taken. A ticker records the longest stretch it was kept from
    /// running while an avatar was decoded and resized.
    #[tokio::test]
    async fn decoding_an_avatar_leaves_the_runtime_free() {
        let state = crate::test_support::offline_state().await;
        let body = crate::libs::image_dp::test_png(2048, 2048);

        let worst = Arc::new(Mutex::new(Duration::ZERO));
        let ticker = tokio::spawn({
            let worst = worst.clone();
            async move {
                let mut last = Instant::now();
                loop {
                    tokio::time::sleep(Duration::from_millis(2)).await;
                    let now = Instant::now();
                    let mut w = worst.lock().unwrap();
                    *w = (*w).max(now - last);
                    last = now;
                }
            }
        });
        tokio::time::sleep(Duration::from_millis(20)).await;

        let started = Instant::now();
        let code = status(&state, "stall-check", body).await;
        let took = started.elapsed();
        ticker.abort();

        // Past normalize: the storage call fails against the closed port.
        assert_eq!(code, 500);
        let worst = *worst.lock().unwrap();
        assert!(
            worst < Duration::from_millis(150),
            "the runtime was blocked for {worst:?} of a {took:?} upload"
        );
    }

    #[tokio::test]
    async fn a_file_over_sixteen_megapixels_is_refused_as_too_large() {
        let state = crate::test_support::offline_state().await;
        assert_eq!(status(&state, "too-large", crate::libs::image_dp::test_png(8192, 2049)).await, 413);
    }

    #[tokio::test]
    async fn one_account_cannot_upload_avatars_without_limit() {
        let state = crate::test_support::offline_state().await;
        for i in 0..AVATAR_UPLOADS_PER_WINDOW {
            assert_eq!(status(&state, "limited", b"not an image".to_vec()).await, 400, "upload {i}");
        }
        assert_eq!(status(&state, "limited", b"not an image".to_vec()).await, 429);
        // Another account is not held back by the first one's uploads.
        assert_eq!(status(&state, "someone-else", b"not an image".to_vec()).await, 400);
    }
}
