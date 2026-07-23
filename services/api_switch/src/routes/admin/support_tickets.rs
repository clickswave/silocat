use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::json;
use sqlx::QueryBuilder;
use crate::{models, routes::respond};

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_tickets))
        .route("/status", post(update_status))
        .route("/{id}", get(get_ticket))
        .route("/{id}/reply", post(reply))
}

async fn get_ticket(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let ticket = sqlx::query_as::<_, models::SupportTicket>("SELECT * FROM support_tickets WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.pg_pool)
        .await;

    let ticket = match ticket {
        Ok(Some(t)) => t,
        Ok(None) => return respond(404, "Ticket not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    };

    let replies = sqlx::query_as::<_, models::SupportReply>(
        "SELECT * FROM support_ticket_replies WHERE ticket_id = $1 ORDER BY created_at ASC",
    )
    .bind(&id)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_default();

    respond(200, "Ticket fetched", vec![], json!({ "ticket": ticket, "replies": replies }))
}

#[derive(Deserialize)]
pub struct ReplyPayload {
    pub body: String,
    pub author_name: Option<String>,
}

async fn reply(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
    Json(payload): Json<ReplyPayload>,
) -> impl IntoResponse {
    let body = payload.body.trim();
    if body.is_empty() {
        return respond(400, "Reply required", vec!["Write a reply.".to_string()], json!({}));
    }
    let author = payload
        .author_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("Admin");

    // Verify ticket exists.
    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM support_tickets WHERE id = $1")
        .bind(&id)
        .fetch_one(&state.pg_pool)
        .await;
    match exists {
        Ok(c) if c > 0 => {}
        Ok(_) => return respond(404, "Ticket not found", vec![], json!({})),
        Err(_e) => return respond(500, "Database error", vec![], json!({})),
    }

    if let Err(_e) = sqlx::query(
        "INSERT INTO support_ticket_replies (ticket_id, author_role, author_name, body) \
         VALUES ($1, 'admin', $2, $3)",
    )
    .bind(&id)
    .bind(author)
    .bind(body)
    .execute(&state.pg_pool)
    .await
    {
        return respond(500, "Failed to add reply", vec![], json!({}));
    }

    // Notify the ticket owner by email (best-effort).
    notify_owner(&state, &id, "reply", body).await;

    respond(200, "Reply added", vec![], json!({}))
}

// Email the ticket owner about an update. Best-effort; failures are logged only.
async fn notify_owner(state: &crate::AppState, ticket_id: &str, kind: &str, excerpt: &str) {
    let row = sqlx::query_as::<_, (String, String, String)>(
        "SELECT email, username, subject FROM support_tickets WHERE id = $1",
    )
    .bind(ticket_id)
    .fetch_optional(&state.pg_pool)
    .await;

    if let Ok(Some((email, username, subject))) = row {
        if !email.trim().is_empty() {
            if let Err(e) = crate::libs::email::send_ticket_update_email(
                &state.smtp_config,
                &username,
                &email,
                ticket_id,
                &subject,
                kind,
                excerpt,
            )
            .await
            {
                println!("Ticket update saved but email failed for {}: {}", ticket_id, e);
            }
        }
    }
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub category: Option<String>, // exact category, or "all"/empty
    pub pro: Option<String>,      // "true" | "false" | anything else = all
    pub status: Option<String>,   // "open" | "closed" | "all"/empty
    pub sort: Option<String>,     // "oldest" => ASC, else newest first
}

async fn list_tickets(
    State(state): State<crate::AppState>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    let mut qb: QueryBuilder<sqlx::Postgres> =
        QueryBuilder::new("SELECT * FROM support_tickets WHERE 1=1");

    if let Some(c) = q.category.as_deref() {
        let c = c.trim();
        if !c.is_empty() && c != "all" {
            qb.push(" AND category = ").push_bind(c.to_string());
        }
    }
    if let Some(s) = q.status.as_deref() {
        let s = s.trim();
        if !s.is_empty() && s != "all" {
            qb.push(" AND status = ").push_bind(s.to_string());
        }
    }
    match q.pro.as_deref() {
        Some("true") => {
            qb.push(" AND is_pro = TRUE");
        }
        Some("false") => {
            qb.push(" AND is_pro = FALSE");
        }
        _ => {}
    }

    if q.sort.as_deref() == Some("oldest") {
        qb.push(" ORDER BY created_at ASC");
    } else {
        qb.push(" ORDER BY created_at DESC");
    }
    qb.push(" LIMIT 1000");

    let tickets = qb
        .build_query_as::<models::SupportTicket>()
        .fetch_all(&state.pg_pool)
        .await;

    match tickets {
        Ok(tickets) => respond(200, "Tickets fetched", vec![], json!({ "tickets": tickets })),
        Err(_e) => respond(500, "Failed to fetch tickets", vec![], json!({})),
    }
}

#[derive(Deserialize)]
pub struct StatusPayload {
    pub id: String,
    pub status: String, // "open" | "closed"
}

async fn update_status(
    State(state): State<crate::AppState>,
    Json(payload): Json<StatusPayload>,
) -> impl IntoResponse {
    if payload.status != "open" && payload.status != "closed" {
        return respond(400, "Invalid status", vec!["Status must be open or closed.".to_string()], json!({}));
    }
    let res = sqlx::query("UPDATE support_tickets SET status = $1 WHERE id = $2")
        .bind(&payload.status)
        .bind(&payload.id)
        .execute(&state.pg_pool)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => {
            // Notify the user when their ticket is resolved (best-effort).
            if payload.status == "closed" {
                notify_owner(&state, &payload.id, "resolved", "").await;
            }
            respond(200, "Ticket updated", vec![], json!({}))
        }
        Ok(_) => respond(404, "Ticket not found", vec![], json!({})),
        Err(_e) => respond(500, "Failed to update ticket", vec![], json!({})),
    }
}
