use axum::{extract::{Path, State}, Json, response::IntoResponse, Router, routing::{delete, get, post}};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{models};
use crate::routes::respond;
use rand::{distributions::Alphanumeric, Rng};

#[derive(Deserialize)]
pub struct CreateInvitePayload {
    pub account_type: String,
    pub description: String,
    pub benefit: String,
}

#[derive(Serialize)]
struct InviteCode {
    code: String,
    description: String,
    account_type: String,
    benefit: String,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    claimed_by: Option<String>,
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_invites).post(create_invite))
        .route("/{code}", delete(delete_invite))
}

async fn list_invites(
    State(state): State<crate::AppState>,
) -> impl IntoResponse {
    let invites = sqlx::query_as!(
        models::InviteCode,
        "SELECT * FROM invite_codes ORDER BY created_on DESC"
    )
    .fetch_all(&state.pg_pool)
    .await;

    match invites {
        Ok(invites) => respond(
            200,
            "Invites fetched successfully",
            vec![],
            json!({ "invites": invites }),
        ),
        Err(_e) => respond(
            500,
            "Failed to fetch invites",
            vec![],
            json!({}),
        ),
    }
}

async fn create_invite(
    State(state): State<crate::AppState>,
    Json(payload): Json<CreateInvitePayload>,
) -> impl IntoResponse {
    let code = {
        let mut rng = rand::thread_rng();

        let part1: String = (0..2)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();
        
        let part2: String = (0..4)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        let part3: String = (0..4)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        let part4: String = (0..4)
            .map(|_| rng.sample(Alphanumeric) as char)
            .collect();

        format!("SC{}-{}-{}-{}", part1, part2, part3, part4).to_uppercase()
    };

    let result = sqlx::query!(
        "INSERT INTO invite_codes (code, description, account_type, benefit) VALUES ($1, $2, $3, $4) RETURNING *",
        code,
        payload.description,
        payload.account_type,
        payload.benefit
    )
    .fetch_one(&state.pg_pool)
    .await;

    match result {
        Ok(record) => respond(
            200,
            "Invite created successfully",
            vec![],
            json!({ 
                "invite": {
                    "code": record.code,
                    "description": record.description,
                    "account_type": record.account_type,
                    "benefit": record.benefit,
                    "created_on": record.created_on,
                    "claimed_by": record.claimed_by
                } 
            }),
        ),
        Err(_e) => respond(
            500,
            "Failed to create invite",
            vec![],
            json!({}),
        ),
    }
}

async fn delete_invite(
    State(state): State<crate::AppState>,
    Path(code): Path<String>,
) -> impl IntoResponse {
    let result = sqlx::query!(
        "DELETE FROM invite_codes WHERE code = $1",
        code
    )
    .execute(&state.pg_pool)
    .await;

    match result {
        Ok(_) => respond(
            200,
            "Invite deleted successfully",
            vec![],
            json!({}),
        ),
        Err(_e) => respond(
            500,
            "Failed to delete invite",
            vec![],
            json!({}),
        ),
    }
}
