pub mod create_folder;
pub mod update_folders;
pub mod delete_folders;
pub mod permanent_delete_folders;
pub mod restore_folders;
pub mod fetch_folder;
pub mod folder_stats;

use axum::routing::post;
use axum::Router;

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/create", post(create_folder::handle))
        .route("/list", post(fetch_folder::handle))
        .route("/update", post(update_folders::handle))
        .route("/delete", post(delete_folders::handle))
        .route("/permanent-delete", post(permanent_delete_folders::handle))
        .route("/restore", post(restore_folders::handle))
        .route("/stats", post(folder_stats::handle))
}
