// Read-only database browser for the admin panel (/home/db-view).
//
//   GET  /admin/db/tables                       -> [{ name, rows }]
//   GET  /admin/db/table?name=&limit=&offset=   -> { columns, rows, limit, offset }
//   POST /admin/db/query  { sql, limit }         -> { columns, rows, count }
//
// `table` validates the table name against pg_stat_user_tables before quoting it,
// so the identifier interpolation is safe. `query` only permits a single
// SELECT/WITH/VALUES statement and runs it inside a READ ONLY transaction, so any
// attempted write errors out at the database. Rows come back as jsonb (to_jsonb),
// values shown raw.
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use crate::routes::respond;

fn default_limit() -> i64 {
    50
}

#[derive(Deserialize)]
pub struct TableParams {
    pub name: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Deserialize)]
pub struct QueryBody {
    pub sql: String,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

pub async fn tables(State(state): State<crate::AppState>) -> impl IntoResponse {
    let rows = sqlx::query_as::<_, (String, i64)>(
        "SELECT relname::text, n_live_tup::int8 FROM pg_stat_user_tables ORDER BY relname",
    )
    .fetch_all(&state.pg_pool)
    .await;

    match rows {
        Ok(rs) => {
            let list: Vec<Value> = rs
                .into_iter()
                .map(|(name, rows)| json!({ "name": name, "rows": rows }))
                .collect();
            respond(200, "Tables", vec![], json!({ "tables": list }))
        }
        Err(e) => respond(500, "Could not list tables", vec![e.to_string()], json!(null)),
    }
}

async fn table_exists(pool: &sqlx::Pool<sqlx::Postgres>, name: &str) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM pg_stat_user_tables WHERE relname = $1)",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub async fn table(
    State(state): State<crate::AppState>,
    Query(p): Query<TableParams>,
) -> impl IntoResponse {
    let limit = p.limit.clamp(1, 1000);
    let offset = p.offset.max(0);

    if !table_exists(&state.pg_pool, &p.name).await {
        return respond(404, "No such table", vec![format!("table '{}' not found", p.name)], json!(null));
    }

    let columns = sqlx::query_scalar::<_, String>(
        "SELECT column_name::text FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = $1 ORDER BY ordinal_position",
    )
    .bind(&p.name)
    .fetch_all(&state.pg_pool)
    .await
    .unwrap_or_default();

    // name is validated above; quote it as an identifier (double any embedded ")
    let safe = p.name.replace('"', "\"\"");
    let q = format!(
        "SELECT to_jsonb(t) AS row FROM (SELECT * FROM \"{}\" LIMIT {} OFFSET {}) t",
        safe, limit, offset
    );
    let rows = sqlx::query_scalar::<_, Value>(&q).fetch_all(&state.pg_pool).await;

    match rows {
        Ok(rs) => respond(
            200,
            "Rows",
            vec![],
            json!({ "columns": columns, "rows": rs, "limit": limit, "offset": offset }),
        ),
        Err(e) => respond(500, "Query failed", vec![e.to_string()], json!(null)),
    }
}

pub async fn query(
    State(state): State<crate::AppState>,
    Json(body): Json<QueryBody>,
) -> impl IntoResponse {
    let limit = body.limit.clamp(1, 1000);
    let sql = body.sql.trim().trim_end_matches(';').trim().to_string();

    if sql.is_empty() {
        return respond(400, "Empty query", vec!["Provide a SELECT query".to_string()], json!(null));
    }
    if sql.contains(';') {
        return respond(400, "One statement only", vec!["Multiple statements are not allowed".to_string()], json!(null));
    }
    let lower = sql.to_ascii_lowercase();
    if !(lower.starts_with("select") || lower.starts_with("with") || lower.starts_with("values") || lower.starts_with("table ")) {
        return respond(400, "Read-only", vec!["Only SELECT / WITH / VALUES queries are allowed".to_string()], json!(null));
    }

    // Run inside a READ ONLY transaction so any write attempt errors at the DB.
    let mut tx = match state.pg_pool.begin().await {
        Ok(t) => t,
        Err(e) => return respond(500, "DB error", vec![e.to_string()], json!(null)),
    };
    if let Err(e) = sqlx::query("SET TRANSACTION READ ONLY").execute(&mut *tx).await {
        return respond(500, "DB error", vec![e.to_string()], json!(null));
    }
    let wrapped = format!("SELECT to_jsonb(t) AS row FROM ( {} ) t LIMIT {}", sql, limit);
    let result = sqlx::query_scalar::<_, Value>(&wrapped).fetch_all(&mut *tx).await;
    let _ = tx.rollback().await;

    match result {
        Ok(rs) => {
            let count = rs.len();
            let columns: Vec<String> = match rs.first() {
                Some(Value::Object(map)) => map.keys().cloned().collect(),
                _ => vec![],
            };
            respond(200, "Query OK", vec![], json!({ "columns": columns, "rows": rs, "count": count }))
        }
        Err(e) => respond(400, "Query error", vec![e.to_string()], json!(null)),
    }
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/tables", get(tables))
        .route("/table", get(table))
        .route("/query", post(query))
}
