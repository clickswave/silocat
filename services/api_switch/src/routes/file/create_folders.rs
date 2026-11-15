use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub name: String,
    pub parent_id: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Owner is the authenticated caller (user or shadow): never a body field.
    let caller = match caller.as_ref() {
        Some(c) => c,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    // Folder names become directories in a downloaded zip: one segment each.
    if let Err(why) = crate::libs::input_validators::entry_name(&payload.name) {
        return respond(400, "Invalid folder name", vec![why], json!({}));
    }

    // If nesting under a parent, the parent must belong to the caller.
    if let Some(parent_id) = &payload.parent_id {
        let parent = sqlx::query!(
            "SELECT user_id, owner_api_key FROM folders WHERE id = $1",
            parent_id
        )
        .fetch_optional(&state.pg_pool)
        .await;
        match parent {
            Ok(Some(p)) if caller.owns(&p.user_id, &p.owner_api_key) => {}
            Ok(_) => return respond(404, "Parent folder not found", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }
    }

    // Same rule as create_files.rs, and as folder/create_folder.rs: decided from
    // the caller, never from the body. False for a drive folder, which is
    // published by the share sheet and nothing else; true for an anonymous drop,
    // which has no drive to sit in and no owner who can log in later to publish
    // it, so the id IS the capability.
    //
    // This is the handler the web app actually reaches (`/file/create-folders`,
    // via ApiServerRoutes.createFolder). The near-identical one behind
    // `/folder/create` is the one that reads like the obvious place for this, so
    // fixing only that left every folder drop made through the UI private while
    // the by-id read path and the 0045 backfill both looked correct.
    let public_access = caller.user_id.is_none();

    let folder = sqlx::query!(
        "INSERT INTO folders (name, user_id, parent_id, owner_api_key, public_access) \
         VALUES ($1, $2, $3, $4, $5) RETURNING id, name, created_on",
        payload.name,
        caller.user_id,
        payload.parent_id,
        Some(caller.api_key.clone()),
        public_access,
    )
    .fetch_one(&state.pg_pool)
    .await;

    match folder {
        Ok(f) => respond(201, "Folder created", vec![], json!({
            "id": f.id,
            "name": f.name,
            "created_on": f.created_on
        })),
        Err(e) => {
            println!("Error creating folder: {:?}", e);
            respond(500, "Failed to create folder", vec![], json!({}))
        }
    }
}

#[cfg(test)]
mod tests {
    //! These call the handler against a Postgres named by
    //! SILOCAT_TEST_DATABASE_URL and skip when it is unset.

    use super::*;
    use crate::test_support::{account, forget, state};
    use axum::http::StatusCode;

    async fn create(st: &crate::AppState, caller: Caller, name: &str) -> StatusCode {
        let payload = Payload { name: name.to_string(), parent_id: None };
        handle(State(st.clone()), Extension(Some(caller)), Json(payload))
            .await
            .into_response()
            .status()
    }

    #[tokio::test]
    async fn a_folder_named_as_a_path_is_refused() {
        // A shared folder's files are zipped under their folder names, so a
        // folder called ".." puts every file in it above the extract folder.
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let mut accepted = vec![];
        for name in ["..", ".", "", "a/../..", "..\\..", "tab\there"] {
            let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
            let status = create(&st, caller, name).await;
            if status != StatusCode::BAD_REQUEST {
                accepted.push((name, status));
            }
        }
        let anon = format!("shadow-{}", crate::libs::rng::uuid());
        let caller = Caller { api_key: anon.clone(), user_id: None, email_verified: true };
        let anon_status = create(&st, caller, "..").await;
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1 OR owner_api_key = $2")
            .bind(&user)
            .bind(&anon)
            .execute(&st.pg_pool)
            .await;
        forget(&st.pg_pool, &user).await;
        assert!(accepted.is_empty(), "accepted: {accepted:?}");
        assert_eq!(anon_status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn an_ordinary_folder_name_is_still_accepted() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
        let status = create(&st, caller, "Photos 2026 (raw)").await;
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(&user).execute(&st.pg_pool).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(status, StatusCode::CREATED);
    }
}
