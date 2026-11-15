use crate::middlewares::resolve_identity::Caller;
use crate::routes::respond;
use axum::{extract::State, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
pub struct Payload {
    pub folder_id: String,
    pub new_name: Option<String>,
    pub new_parent_id: Option<String>,
    pub move_to_root: Option<bool>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(caller): Extension<Option<Caller>>,
    Json(payload): Json<Payload>,
) -> impl IntoResponse {
    // Identity comes from the authenticated X-Api-Key, never the request body.
    let user_id = match caller.as_ref().and_then(|c| c.user_id.clone()) {
        Some(uid) => uid,
        None => {
            return respond(
                401,
                "Unauthorized",
                vec!["Authentication required".to_string()],
                json!({}),
            )
        }
    };

    if let Some(name) = &payload.new_name {
        if let Err(why) = crate::libs::input_validators::entry_name(name) {
            return respond(400, "Invalid folder name", vec![why], json!({}));
        }
    }

    // Verify ownership and update
    if let Some(parent_id) = &payload.new_parent_id {
        if parent_id == &payload.folder_id {
            return respond(400, "Cannot move folder into itself", vec![], json!({}));
        }

        // The destination parent must belong to the caller.
        let parent_check = sqlx::query_scalar!(
            "SELECT id FROM folders WHERE id = $1 AND user_id = $2",
            parent_id,
            user_id
        )
        .fetch_optional(&state.pg_pool)
        .await;

        match parent_check {
            Ok(Some(_)) => {}
            Ok(None) => {
                return respond(404, "New parent folder not found", vec![], json!({}))
            }
            Err(_e) => {
                return respond(500, "Database error", vec![], json!({}))
            }
        }
    }

    // Using COALESCE to only update fields that are provided (not None/NULL)
    // AND handling explicit move to root via move_to_root flag
    let result = sqlx::query(
        "UPDATE folders SET name = COALESCE($1, name), parent_id = CASE WHEN $5 = true THEN NULL ELSE COALESCE($2, parent_id) END WHERE id = $3 AND user_id = $4",
    )
    .bind(payload.new_name)
    .bind(payload.new_parent_id)
    .bind(payload.folder_id)
    .bind(user_id)
    .bind(payload.move_to_root)
    .execute(&state.pg_pool)
    .await;

    match result {
        Ok(res) => {
            if res.rows_affected() > 0 {
                respond(200, "Folder updated", vec![], json!({}))
            } else {
                 respond(404, "Folder not found or access denied", vec![], json!({}))
            }
        },
        Err(e) => {
             println!("Error updating folder: {:?}", e);
             respond(500, "Failed to update folder", vec![], json!({}))
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
    async fn renaming_a_folder_to_a_path_is_refused() {
        let Some(st) = state().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let (user, key) = account(&st.pg_pool).await;
        let folder_id: String = sqlx::query_scalar(
            "INSERT INTO folders (name, user_id, owner_api_key) VALUES ('Photos', $1, $2) RETURNING id",
        )
        .bind(&user)
        .bind(&key)
        .fetch_one(&st.pg_pool)
        .await
        .expect("seed folder");
        let mut seen = vec![];
        for name in ["..", "../../x", "Holiday"] {
            let caller = Caller { api_key: key.clone(), user_id: Some(user.clone()), email_verified: true };
            let payload = Payload {
                folder_id: folder_id.clone(),
                new_name: Some(name.to_string()),
                new_parent_id: None,
                move_to_root: None,
            };
            let status = handle(State(st.clone()), Extension(Some(caller)), Json(payload))
                .await
                .into_response()
                .status();
            let now: String = sqlx::query_scalar("SELECT name FROM folders WHERE id = $1")
                .bind(&folder_id)
                .fetch_one(&st.pg_pool)
                .await
                .unwrap();
            seen.push((name, status, now));
        }
        let _ = sqlx::query("DELETE FROM folders WHERE user_id = $1").bind(&user).execute(&st.pg_pool).await;
        forget(&st.pg_pool, &user).await;
        assert_eq!(
            seen,
            vec![
                ("..", StatusCode::BAD_REQUEST, "Photos".to_string()),
                ("../../x", StatusCode::BAD_REQUEST, "Photos".to_string()),
                ("Holiday", StatusCode::OK, "Holiday".to_string()),
            ]
        );
    }
}
