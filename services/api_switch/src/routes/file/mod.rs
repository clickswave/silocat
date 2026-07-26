mod create_files;
mod mark_chunk_complete;
mod fetch_files;
mod delete_files;
mod restore_files;
mod permanent_delete_files;
mod list_files;
mod update_files;

mod create_folders;
mod fetch_folders;
mod delete_folders;

mod fetch_chunks;
mod fetch_progress;
mod fetch_resource;
mod star;
mod share;



use axum::Router;

pub fn router(state: crate::AppState) -> Router<crate::AppState> {
    Router::new()
        .route("/create-files", axum::routing::post(create_files::handle))
        .route("/mark-chunk-complete", axum::routing::post(mark_chunk_complete::handle))
        .route("/fetch-files", axum::routing::post(fetch_files::handle))
        .route("/delete-files", axum::routing::post(delete_files::handle))
        .route("/restore-files", axum::routing::post(restore_files::handle))
        .route("/permanent-delete-files", axum::routing::post(permanent_delete_files::handle))
        .route("/list-files", axum::routing::post(list_files::handle))
        .route("/update-files", axum::routing::post(update_files::handle))
        .route("/create-folders", axum::routing::post(create_folders::handle))
        .route("/fetch-folders", axum::routing::post(fetch_folders::handle))
        .route("/delete-folders", axum::routing::post(delete_folders::handle))
        .route("/fetch-chunks", axum::routing::post(fetch_chunks::handle))
        .route("/fetch-resource", axum::routing::post(fetch_resource::handle))
        .route("/star/file", axum::routing::post(star::file))
        .route("/star/folder", axum::routing::post(star::folder))
        
        .route("/share/toggle", axum::routing::post(share::toggle_share))
        .route("/share/regenerate", axum::routing::post(share::regenerate_token))
        .route("/share/info/{id}", axum::routing::get(share::get_share_info))
        
        // Public routes (no auth middleware check if placed outside? No, router() is wrapped in auth check in main.rs!)
        // `authority_sign_check` validates the `X-Authority-Sign` header from the web server.
        // The web server (SvelteKit) HAS the authority sign.
        // So anonymous users hitting SvelteKit -> SvelteKit hits Backend WITH Sign.
        // So backend is technically "authenticated" by the web server service, but "UserTokenData" extension won't be present if we don't extract it.
        // My `public_get_info` and `public_authorize_download` do NOT take `Extension<UserTokenData>`, so they are fine!
        
        .route("/public/share/info/{token}", axum::routing::get(share::public_get_info))
        .route("/public/share/authorize", axum::routing::post(share::public_authorize_download))
        .route("/public/share/fetch-chunks", axum::routing::post(share::public_fetch_file_chunks))
        // .route("/fetch-progress", axum::routing::post(fetch_progress::handle))
        // Resolve the X-Api-Key into an Option<Caller> (user or shadow) for every
        // file route. Non-rejecting: public/share routes simply see no caller.
        // Handlers derive ownership from the Caller, never from body user_id.
        .layer(axum::middleware::from_fn_with_state(
            state,
            crate::middlewares::resolve_identity::resolve_identity,
        ))
}