mod register_personal;
mod login;
mod change_password;
mod update_profile;
mod fetch_storage_stats;
mod fetch_info;
mod verify_email;
mod resend_verification;
mod google_auth;

use axum::routing::post;
use axum::routing::get;
use axum::Router;

pub fn router(state: crate::AppState) -> Router<crate::AppState> {
    let public_routes = Router::new()
        .route("/register-personal", post(register_personal::handle))
        .route("/login", post(login::handle))
        .route("/storage-stats", post(fetch_storage_stats::handle))
        .route("/google-auth", post(google_auth::handle));

    let protected_routes = Router::new()
        .route("/change-password", post(change_password::handle))
        .route("/update-profile", post(update_profile::handle))
        .route("/info", get(fetch_info::handle))
        .route("/verify-email", post(verify_email::handle))
        .route("/resend-verification", post(resend_verification::handle))

        .route_layer(
            axum::middleware::from_fn_with_state(
                state.clone(),
                crate::middlewares::validate_token::validate_token,
            )
        );

    public_routes.merge(protected_routes)
}