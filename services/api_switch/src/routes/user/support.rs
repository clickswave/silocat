use axum::{extract::{Path, State}, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use serde_json::json;
use crate::{libs, models, models::UserTokenData, routes::respond};

#[derive(Deserialize)]
pub struct Input {
    pub category: Option<String>,
    pub subject: String,
    pub message: String,
    pub email: Option<String>,
}

pub async fn handle(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Json(payload): Json<Input>,
) -> impl IntoResponse {
    let subject = payload.subject.trim();
    let message = payload.message.trim();

    if subject.len() < 3 {
        return respond(400, "Subject too short", vec!["Please add a short subject.".to_string()], json!({}));
    }
    if message.len() < 5 {
        return respond(400, "Message too short", vec!["Please write a bit more detail.".to_string()], json!({}));
    }
    if message.len() > 5000 {
        return respond(400, "Message too long", vec!["Please keep it under 5000 characters.".to_string()], json!({}));
    }

    let category = payload.category.as_deref().unwrap_or("other").trim();
    // Prefer the reply address the user typed; fall back to their account email.
    let reply_email = payload
        .email
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .unwrap_or(&user.email);

    // Snapshot whether they were a Pro member when they wrote in.
    let is_pro = user
        .subscription
        .as_ref()
        .map(|s| s.name == "Pro")
        .unwrap_or(false);

    // Persist the ticket — this is the source of truth (admin panel reads these).
    if let Err(e) = sqlx::query(
        "INSERT INTO support_tickets (user_id, username, email, category, subject, message, is_pro) \
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&user.id)
    .bind(&user.username)
    .bind(reply_email)
    .bind(category)
    .bind(subject)
    .bind(message)
    .bind(is_pro)
    .execute(&state.pg_pool)
    .await
    {
        return respond(500, "Could not submit your message", vec![e.to_string()], json!({}));
    }

    // Email notification is best-effort: the ticket is already saved.
    if let Err(e) = libs::email::send_support_email(
        &state.smtp_config,
        &user.username,
        reply_email,
        category,
        subject,
        message,
    )
    .await
    {
        println!("Support ticket saved but email notification failed: {}", e);
    }

    respond(200, "Message sent", vec![], json!({}))
}

// List the signed-in user's own tickets (newest first).
pub async fn list(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
) -> impl IntoResponse {
    let tickets = sqlx::query_as::<_, models::SupportTicket>(
        "SELECT * FROM support_tickets WHERE user_id = $1 ORDER BY created_at DESC LIMIT 100",
    )
    .bind(&user.id)
    .fetch_all(&state.pg_pool)
    .await;

    match tickets {
        Ok(tickets) => respond(200, "Tickets fetched", vec![], json!({ "tickets": tickets })),
        Err(e) => respond(500, "Failed to fetch tickets", vec![e.to_string()], json!({})),
    }
}

// One of the user's tickets + its reply thread. 404 if not theirs.
pub async fn get_one(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let ticket = sqlx::query_as::<_, models::SupportTicket>(
        "SELECT * FROM support_tickets WHERE id = $1 AND user_id = $2",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_optional(&state.pg_pool)
    .await;

    let ticket = match ticket {
        Ok(Some(t)) => t,
        Ok(None) => return respond(404, "Ticket not found", vec![], json!({})),
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
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
pub struct ReplyInput {
    pub body: String,
}

// Add a reply (as the user) to one of their tickets; reopens a closed ticket.
pub async fn reply(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Path(id): Path<String>,
    Json(payload): Json<ReplyInput>,
) -> impl IntoResponse {
    let body = payload.body.trim();
    if body.len() < 2 {
        return respond(400, "Reply too short", vec!["Please write a message.".to_string()], json!({}));
    }
    if body.len() > 5000 {
        return respond(400, "Reply too long", vec!["Please keep it under 5000 characters.".to_string()], json!({}));
    }

    // Ownership check.
    let owns = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM support_tickets WHERE id = $1 AND user_id = $2",
    )
    .bind(&id)
    .bind(&user.id)
    .fetch_one(&state.pg_pool)
    .await;
    match owns {
        Ok(c) if c > 0 => {}
        Ok(_) => return respond(404, "Ticket not found", vec![], json!({})),
        Err(e) => return respond(500, "Database error", vec![e.to_string()], json!({})),
    }

    if let Err(e) = sqlx::query(
        "INSERT INTO support_ticket_replies (ticket_id, author_role, author_name, body) \
         VALUES ($1, 'user', $2, $3)",
    )
    .bind(&id)
    .bind(&user.username)
    .bind(body)
    .execute(&state.pg_pool)
    .await
    {
        return respond(500, "Failed to add reply", vec![e.to_string()], json!({}));
    }

    // A user reply reopens the ticket.
    let _ = sqlx::query("UPDATE support_tickets SET status = 'open' WHERE id = $1")
        .bind(&id)
        .execute(&state.pg_pool)
        .await;

    respond(200, "Reply added", vec![], json!({}))
}

#[derive(Deserialize)]
pub struct StatusInput {
    pub status: String,
}

// Let the owner close or reopen their own ticket.
pub async fn set_status(
    State(state): State<crate::AppState>,
    Extension(user): Extension<UserTokenData>,
    Path(id): Path<String>,
    Json(payload): Json<StatusInput>,
) -> impl IntoResponse {
    if payload.status != "open" && payload.status != "closed" {
        return respond(400, "Invalid status", vec!["Status must be open or closed.".to_string()], json!({}));
    }
    let res = sqlx::query("UPDATE support_tickets SET status = $1 WHERE id = $2 AND user_id = $3")
        .bind(&payload.status)
        .bind(&id)
        .bind(&user.id)
        .execute(&state.pg_pool)
        .await;
    match res {
        Ok(r) if r.rows_affected() > 0 => respond(200, "Ticket updated", vec![], json!({})),
        Ok(_) => respond(404, "Ticket not found", vec![], json!({})),
        Err(e) => respond(500, "Failed to update ticket", vec![e.to_string()], json!({})),
    }
}
