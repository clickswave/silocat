use crate::middlewares::resolve_identity::Caller;
use crate::models;
use crate::routes::respond;
use axum::extract::State;
use axum::response::IntoResponse;
use axum::{Extension, Json};
use serde_json::json;

#[derive(serde::Deserialize, Debug)]
pub struct PayloadBody {
    pub name: String,
    pub parent_id: Option<String>,
    pub uploaded_as_files: Option<bool>,
}

pub async fn handle(
    State(axum_state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<PayloadBody>,
) -> impl IntoResponse {
    // Identity + ownership come from the authenticated X-Api-Key, never the body.
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

    let uploaded_as_files = payload.uploaded_as_files.unwrap_or(false);

    // If a parent is requested, it must belong to the caller, by the same rule
    // as every other ownership check (Caller::owns). Matching an anonymous
    // caller on owner_api_key alone let a rotated-out account key, registered
    // as an anonymous key, nest folders inside the account's folders.
    if let Some(parent_id) = &payload.parent_id {
        let parent = sqlx::query!(
            "SELECT user_id, owner_api_key FROM folders WHERE id = $1",
            parent_id
        )
        .fetch_optional(&axum_state.pg_pool)
        .await;
        match parent {
            Ok(Some(p)) if caller.owns(&p.user_id, &p.owner_api_key) => {}
            Ok(_) => return respond(404, "Parent folder not found", vec![], json!({})),
            Err(_e) => return respond(500, "Database error", vec![], json!({})),
        }
    }

    // Visibility is decided from the caller, never from the body, and by exactly
    // the rule create_files.rs applies to a file.
    //
    // A folder in a registered user's drive is private on creation: publishing it
    // is the share sheet's job (share::toggle_share), which the owner drives and
    // can turn back off. An anonymous drop is the mirror image: it has no drive
    // to sit in and no owner who can log in later to publish it, so the id IS the
    // capability and the folder has to be readable by it. Without this the upload
    // page handed out a /<folder_id> link that 404s for everyone it was sent to.
    let public_access = caller.user_id.is_none();

    let insert_query = sqlx::query_as!(
        models::Folder,
        "INSERT INTO folders (name, user_id, parent_id, uploaded_as_files, owner_api_key, public_access)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        payload.name,
        caller.user_id.clone(),
        payload.parent_id,
        uploaded_as_files,
        Some(caller.api_key.clone()),
        public_access
    )
    .fetch_one(&axum_state.pg_pool)
    .await;

    match insert_query {
        Ok(folder) => {
            respond(
                200,
                "Folder created",
                vec![],
                json!({ "folder": folder }),
            )
        },
        Err(_e) => {
             respond(
                500,
                "Failed to create folder",
                vec![],
                json!({}),
            )
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

    #[tokio::test]
    async fn a_folder_named_as_a_path_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let mut accepted = vec![];
        for name in ["..", "a/b", "Photos"] {
            let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
            let payload = PayloadBody { name: name.to_string(), parent_id: None, uploaded_as_files: None };
            let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload))
                .await
                .into_response()
                .status();
            accepted.push((name, status));
        }
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(&user).execute(&st.pg_pool).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(
            accepted,
            vec![("..", StatusCode::BAD_REQUEST), ("a/b", StatusCode::BAD_REQUEST), ("Photos", StatusCode::OK)]
        );
    }
}
