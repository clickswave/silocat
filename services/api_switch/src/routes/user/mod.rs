mod register_personal;
mod login;
mod change_password;
mod update_profile;
mod fetch_storage_stats;
mod fetch_info;
mod verify_email;
mod resend_verification;
mod request_email_change;
mod confirm_email_change;
mod support;
mod google_auth;
mod avatar;
mod forgot_password;
mod reset_password;
mod username_status;

use axum::routing::post;
use axum::routing::get;
use axum::Router;

pub fn router(state: crate::AppState) -> Router<crate::AppState> {
    let public_routes = Router::new()
        .route("/register-personal", post(register_personal::handle))
        .route("/login", post(login::handle))
        .route("/storage-stats", post(fetch_storage_stats::handle))
        .route("/google-auth", post(google_auth::handle))
        // Public OTP-based password reset (no session yet).
        .route("/forgot-password", post(forgot_password::handle))
        .route("/reset-password", post(reset_password::handle))
        // Avatar bytes are served via a presigned-GET redirect; the lookup is
        // service-signed (web_server proxy), so it lives among the public routes.
        .route("/avatar-url", get(avatar::presigned));

    let protected_routes = Router::new()
        .route("/avatar", post(avatar::upload).delete(avatar::remove))
        .route("/change-password", post(change_password::handle))
        .route("/update-profile", post(update_profile::handle))
        .route("/info", get(fetch_info::handle))
        .route("/username-status", get(username_status::handle))
        .route("/verify-email", post(verify_email::handle))
        .route("/resend-verification", post(resend_verification::handle))
        .route("/request-email-change", post(request_email_change::handle))
        .route("/confirm-email-change", post(confirm_email_change::handle))
        .route("/support", post(support::handle).get(support::list))
        .route("/support/{id}", get(support::get_one))
        .route("/support/{id}/reply", post(support::reply))
        .route("/support/{id}/status", post(support::set_status))

        .route_layer(
            axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::validate_token::validate_token,
            )
        );

    public_routes.merge(protected_routes)
}