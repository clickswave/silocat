use axum::{
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use futures_util::stream::{self, StreamExt};
use serde::Deserialize;
use serde_json::json;
use sqlx::types::chrono::{DateTime, Utc};
use crate::routes::respond;

/// How long an anonymous drop lives before watchcat reaps it. Read from the
/// same variable watchcat uses, so the console's "expires in" matches the reap.
fn shadow_ttl_days() -> i64 {
    std::env::var("WATCHCAT_SHADOW_TTL_DAYS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(7)
        .max(1)
}

/// Every live file with what the console derives from it: a type category,
/// the state of its share link, whether it is a stalled upload, and whether an
/// open abuse report points at it. Placeholder $1 is the anonymous TTL in days.
///
/// Link state: an anonymous drop is public by its id, by design, so it counts
/// as an active link while `public_access` holds. An owned file's link is
/// active while its share is on and not past its expiry; a one-time link is
/// also used up once downloaded, the cap share.rs enforces only for 'once'.
const FILES_CTE: &str = "\
WITH fx AS ( \
    SELECT f.id, f.user_id, f.name, f.mime, f.size, f.encrypted, f.created_on, f.downloads, \
        f.public_access, f.share_token, f.share_type, f.share_expires_at, f.link_downloads, f.link_max_downloads, \
        f.total_chunks, f.uploaded_chunks, f.folder_id, f.sha256_checksum, \
        COALESCE(u.username, u.email) AS owner, \
        CASE \
            WHEN f.mime LIKE 'video/%' THEN 'video' \
            WHEN f.mime LIKE 'image/%' THEN 'image' \
            WHEN f.mime LIKE 'audio/%' THEN 'audio' \
            WHEN lower(f.name) ~ '\\.(zip|tar|gz|tgz|7z|rar|bz2|xz|zst)$' OR f.mime IN ('application/zip','application/x-tar','application/gzip','application/x-7z-compressed','application/x-rar-compressed','application/x-bzip2','application/x-xz') THEN 'archive' \
            WHEN f.mime LIKE 'text/%' OR f.mime = 'application/pdf' OR f.mime LIKE '%officedocument%' OR f.mime LIKE 'application/msword%' OR f.mime LIKE '%opendocument%' OR f.mime = 'application/rtf' THEN 'document' \
            ELSE 'other' END AS cat, \
        CASE \
            WHEN f.user_id IS NULL THEN CASE WHEN f.public_access THEN 'active' ELSE 'off' END \
            WHEN f.share_type IS DISTINCT FROM 'off' AND f.share_token IS NOT NULL THEN \
                CASE WHEN (f.share_expires_at IS NOT NULL AND f.share_expires_at <= NOW()) \
                       OR (f.share_type = 'once' AND COALESCE(f.link_downloads, 0) >= COALESCE(f.link_max_downloads, 1)) \
                     THEN 'expired' ELSE 'active' END \
            ELSE 'off' END AS link, \
        (f.uploaded_chunks < f.total_chunks) AS stalled, \
        CASE WHEN f.user_id IS NULL THEN f.created_on + make_interval(days => $1::int) END AS expires_at, \
        EXISTS (SELECT 1 FROM request_uploads ru WHERE ru.file_id = f.id) AS via_request, \
        EXISTS (SELECT 1 FROM abuse_reports ar WHERE ar.status = 'open' \
                AND (ar.share_token = f.share_token OR ar.share_token = f.id)) AS reported \
    FROM files f LEFT JOIN users u ON u.id = f.user_id \
    WHERE f.deleted = false)";

/// The console's filters over `fx`. Fixed strings, never user input.
fn file_filter(f: &str) -> Option<&'static str> {
    Some(match f {
        "all" => "TRUE",
        "public" => "link = 'active'",
        "encrypted" => "encrypted",
        "anon" => "user_id IS NULL",
        "expiring" => "user_id IS NULL AND expires_at > NOW() AND expires_at <= NOW() + INTERVAL '24 hours'",
        "request" => "via_request",
        "stalled" => "stalled",
        "reported" => "reported",
        _ => return None,
    })
}
const FILE_FILTERS: [&str; 8] = ["all", "public", "encrypted", "anon", "expiring", "request", "stalled", "reported"];
const CATEGORIES: [&str; 6] = ["video", "archive", "image", "audio", "document", "other"];

fn default_files_limit() -> i64 {
    100
}

#[derive(Deserialize)]
pub struct ListParams {
    #[serde(default = "default_files_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    // Search across every file, not just the page the panel has loaded: a
    // case-insensitive substring of the file name or the owner's username or
    // email, or an exact file id or user id.
    #[serde(default)]
    pub q: Option<String>,
    #[serde(default)]
    pub filter: Option<String>,
    /// video, archive, image, audio, document, other
    #[serde(default)]
    pub cat: Option<String>,
    /// new (default), size, dls
    #[serde(default)]
    pub sort: Option<String>,
}

// The search clause over `fx`: $2 is the ILIKE pattern, $3 the bare term; both
// bound NULL means no search.
const FILE_SEARCH: &str = "($2::text IS NULL OR name ILIKE $2 ESCAPE '\\' OR owner ILIKE $2 ESCAPE '\\' \
     OR id = $3 OR user_id = $3)";

fn file_json(r: &sqlx::postgres::PgRow) -> serde_json::Value {
    use sqlx::Row;
    json!({
        "id": r.get::<String, _>("id"),
        "user_id": r.try_get::<Option<String>, _>("user_id").ok().flatten(),
        "owner": r.try_get::<Option<String>, _>("owner").ok().flatten(),
        "name": r.get::<String, _>("name"),
        "mime": r.get::<String, _>("mime"),
        "size": r.get::<i64, _>("size"),
        "encrypted": r.get::<bool, _>("encrypted"),
        "created_on": r.get::<DateTime<Utc>, _>("created_on"),
        "downloads": r.get::<i64, _>("downloads"),
        "public_access": r.get::<bool, _>("public_access"),
        "share_token": r.try_get::<Option<String>, _>("share_token").ok().flatten(),
        "share_type": r.try_get::<Option<String>, _>("share_type").ok().flatten(),
        "share_expires_at": r.try_get::<Option<DateTime<Utc>>, _>("share_expires_at").ok().flatten(),
        "link_downloads": r.try_get::<Option<i64>, _>("link_downloads").ok().flatten(),
        "link_max_downloads": r.try_get::<Option<i64>, _>("link_max_downloads").ok().flatten(),
        "total_chunks": r.try_get::<Option<i32>, _>("total_chunks").ok().flatten(),
        "uploaded_chunks": r.try_get::<Option<i32>, _>("uploaded_chunks").ok().flatten(),
        "sha256": r.try_get::<Option<String>, _>("sha256_checksum").ok().flatten(),
        "cat": r.get::<String, _>("cat"),
        "link": r.get::<String, _>("link"),
        "stalled": r.try_get::<Option<bool>, _>("stalled").ok().flatten().unwrap_or(false),
        "expires_at": r.try_get::<Option<DateTime<Utc>>, _>("expires_at").ok().flatten(),
        "via_request": r.get::<bool, _>("via_request"),
        "reported": r.get::<bool, _>("reported"),
        "folder": r.try_get::<Option<String>, _>("folder").ok().flatten(),
        "last_download": r.try_get::<Option<DateTime<Utc>>, _>("last_download").ok().flatten(),
        "last_chunk_at": r.try_get::<Option<DateTime<Utc>>, _>("last_chunk_at").ok().flatten(),
        "request_from": r.try_get::<Option<String>, _>("request_from").ok().flatten(),
        "request_label": r.try_get::<Option<String>, _>("request_label").ok().flatten(),
        "report_reason": r.try_get::<Option<String>, _>("report_reason").ok().flatten(),
        "report_count": r.try_get::<Option<i64>, _>("report_count").ok().flatten().unwrap_or(0),
    })
}

/// Per-row detail for the rows on the page only: the folder path, the last
/// link open, when the last chunk landed, who delivered it through a request
/// link, and the open reports against it.
const ROW_DETAIL: &str = "\
    (SELECT string_agg(p.name, '/' ORDER BY p.depth DESC) FROM ( \
        WITH RECURSIVE p(id, name, parent_id, depth) AS ( \
            SELECT fo.id, fo.name, fo.parent_id, 0 FROM folders fo WHERE fo.id = fx.folder_id \
            UNION ALL SELECT fo.id, fo.name, fo.parent_id, p.depth + 1 FROM folders fo JOIN p ON fo.id = p.parent_id WHERE p.depth < 32 \
        ) SELECT * FROM p) p) AS folder, \
    (SELECT MAX(e.created_at) FROM share_access_events e WHERE e.file_id = fx.id) AS last_download, \
    CASE WHEN fx.stalled THEN (SELECT MAX(c.created_on) FROM chunks c WHERE c.file_id = fx.id) END AS last_chunk_at, \
    (SELECT ru.uploader_name FROM request_uploads ru WHERE ru.file_id = fx.id LIMIT 1) AS request_from, \
    (SELECT fr.label FROM request_uploads ru JOIN file_requests fr ON fr.id = ru.request_id WHERE ru.file_id = fx.id LIMIT 1) AS request_label, \
    (SELECT ar.reason FROM abuse_reports ar WHERE ar.status = 'open' AND (ar.share_token = fx.share_token OR ar.share_token = fx.id) ORDER BY ar.created_at DESC LIMIT 1) AS report_reason, \
    (SELECT COUNT(*) FROM abuse_reports ar WHERE ar.status = 'open' AND (ar.share_token = fx.share_token OR ar.share_token = fx.id))::BIGINT AS report_count";

pub async fn list_files(
    State(state): State<crate::AppState>,
    Query(p): Query<ListParams>,
) -> impl IntoResponse {
    use sqlx::Row;
    // Server-side pagination so the admin panel can page past the first screen
    // of files instead of being capped at a hardcoded 100 with no way forward.
    let limit = p.limit.clamp(1, 500);
    let offset = p.offset.max(0);
    let filter = p.filter.as_deref().unwrap_or("all");
    let Some(fsql) = file_filter(filter) else {
        return respond(400, "Unknown filter", vec![], json!({}));
    };
    let cat = p.cat.as_deref().filter(|c| CATEGORIES.contains(c)).map(str::to_string);
    let order = match p.sort.as_deref().unwrap_or("new") {
        "size" => "size DESC",
        "dls" => "downloads DESC",
        _ => "created_on DESC",
    };

    let q = p.q.as_deref().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
    // Escape LIKE metacharacters so a search for "a_b" matches that literally.
    let pattern = q.as_ref().map(|s| {
        format!("%{}%", s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_"))
    });
    let ttl = shadow_ttl_days() as i32;

    // Runtime queries (not the compile-time macro) so the filters can be
    // composed without regenerating the sqlx offline cache.
    let files = sqlx::query(&format!(
        "{FILES_CTE} SELECT fx.*, {ROW_DETAIL} FROM fx \
         WHERE {FILE_SEARCH} AND ({fsql}) AND ($4::text IS NULL OR cat = $4) \
         ORDER BY {order}, id LIMIT $5 OFFSET $6"
    ))
    .bind(ttl)
    .bind(&pattern)
    .bind(&q)
    .bind(&cat)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg_pool)
    .await;

    // Each filter's count under the current search and type, for the chips.
    let counts_sql = FILE_FILTERS
        .iter()
        .map(|f| format!("COUNT(*) FILTER (WHERE {})::BIGINT AS \"{f}\"", file_filter(f).unwrap_or("FALSE")))
        .collect::<Vec<_>>()
        .join(", ");
    let counts = sqlx::query(&format!(
        "{FILES_CTE} SELECT {counts_sql} FROM fx WHERE {FILE_SEARCH} AND ($4::text IS NULL OR cat = $4)"
    ))
    .bind(ttl)
    .bind(&pattern)
    .bind(&q)
    .bind(&cat)
    .fetch_one(&state.pg_pool)
    .await;

    match (files, counts) {
        (Ok(files), Ok(c)) => {
            let counts: serde_json::Map<String, serde_json::Value> = FILE_FILTERS
                .iter()
                .map(|f| (f.to_string(), json!(c.try_get::<i64, _>(*f).unwrap_or(0))))
                .collect();
            let total = counts.get(filter).cloned().unwrap_or(json!(0));
            respond(
                200,
                "Files retrieved successfully",
                vec![],
                json!({
                    "files": files.iter().map(file_json).collect::<Vec<_>>(),
                    "counts": counts,
                    "total": total,
                    "limit": limit,
                    "offset": offset,
                    "q": q,
                    "filter": filter,
                    "cat": cat,
                    "ttl_days": ttl,
                }),
            )
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("[admin] files list: {e:?}");
            respond(500, "Failed to retrieve files", vec![], json!({}))
        }
    }
}

/// GET /admin/files/summary: the numbers above the Files list. Stored bytes are
/// measured from chunks, the way quotas are; the type breakdown covers every
/// live file, not just the page on screen.
async fn summary(State(state): State<crate::AppState>) -> impl IntoResponse {
    use sqlx::Row;
    let ttl = shadow_ttl_days() as i32;
    let totals = sqlx::query(&format!(
        "{FILES_CTE} SELECT \
            COUNT(*)::BIGINT AS files, \
            COALESCE(SUM(size), 0)::BIGINT AS bytes, \
            COALESCE(SUM(size) FILTER (WHERE encrypted), 0)::BIGINT AS enc_bytes, \
            COUNT(*) FILTER (WHERE link = 'active')::BIGINT AS public_links, \
            COUNT(*) FILTER (WHERE user_id IS NULL)::BIGINT AS anon, \
            COUNT(*) FILTER (WHERE user_id IS NULL AND expires_at > NOW() AND expires_at <= NOW() + INTERVAL '24 hours')::BIGINT AS expiring, \
            COUNT(*) FILTER (WHERE reported)::BIGINT AS reported, \
            COUNT(*) FILTER (WHERE stalled)::BIGINT AS stalled, \
            COUNT(DISTINCT user_id)::BIGINT AS owners \
         FROM fx"
    ))
    .bind(ttl)
    .fetch_one(&state.pg_pool)
    .await;
    let stored = sqlx::query(
        "SELECT COALESCE(SUM(c.size_on_server), 0)::BIGINT AS stored, \
                COALESCE(SUM(c.size_on_server) FILTER (WHERE c.created_on >= date_trunc('day', NOW())), 0)::BIGINT AS today \
         FROM chunks c JOIN files f ON f.id = c.file_id WHERE f.deleted = false AND c.uploaded = true",
    )
    .fetch_one(&state.pg_pool)
    .await;
    let cats = sqlx::query(&format!(
        "{FILES_CTE} SELECT cat, COUNT(*)::BIGINT AS n, COALESCE(SUM(size), 0)::BIGINT AS bytes FROM fx GROUP BY cat"
    ))
    .bind(ttl)
    .fetch_all(&state.pg_pool)
    .await;
    match (totals, stored, cats) {
        (Ok(t), Ok(s), Ok(c)) => respond(
            200,
            "Files summary",
            vec![],
            json!({
                "files": t.get::<i64, _>("files"),
                "bytes": t.get::<i64, _>("bytes"),
                "enc_bytes": t.get::<i64, _>("enc_bytes"),
                "public_links": t.get::<i64, _>("public_links"),
                "anon": t.get::<i64, _>("anon"),
                "expiring": t.get::<i64, _>("expiring"),
                "reported": t.get::<i64, _>("reported"),
                "stalled": t.get::<i64, _>("stalled"),
                "owners": t.get::<i64, _>("owners"),
                "stored": s.get::<i64, _>("stored"),
                "stored_today": s.get::<i64, _>("today"),
                "ttl_days": ttl,
                "categories": c.iter().map(|r| json!({
                    "cat": r.get::<String, _>("cat"),
                    "n": r.get::<i64, _>("n"),
                    "bytes": r.get::<i64, _>("bytes"),
                })).collect::<Vec<_>>(),
            }),
        ),
        _ => respond(500, "Failed to summarise files", vec![], json!({})),
    }
}

/// POST /admin/files/{id}/disable-link: the share goes off the way the owner
/// turns it off (type off, token dropped, `public_access` cleared, so neither
/// the link nor the id path serves it). The file stays stored.
async fn disable_link(State(state): State<crate::AppState>, Path(id): Path<String>) -> impl IntoResponse {
    let r = sqlx::query(
        "UPDATE files SET share_type = 'off', share_token = NULL, public_access = false \
         WHERE id = $1 AND deleted = false",
    )
    .bind(&id)
    .execute(&state.pg_pool)
    .await;
    match r {
        Ok(r) if r.rows_affected() == 0 => respond(404, "File not found", vec![], json!({})),
        Ok(_) => respond(200, "Link disabled", vec![], json!({})),
        Err(_) => respond(500, "Failed to disable the link", vec![], json!({})),
    }
}

#[derive(Deserialize)]
pub struct DeleteInput {
    ids: Vec<String>,
}

/// POST /admin/files/delete: move files to the trash with their links off.
/// Watchcat's trash retention removes the stored bytes on its schedule, the
/// same path an owner's delete takes, so there is one way bytes leave storage.
async fn delete_files(State(state): State<crate::AppState>, Json(body): Json<DeleteInput>) -> impl IntoResponse {
    if body.ids.is_empty() || body.ids.len() > 500 {
        return respond(400, "Send between 1 and 500 file ids", vec![], json!({}));
    }
    let r = sqlx::query(
        "UPDATE files SET deleted = true, deleted_on = NOW(), share_type = 'off', share_token = NULL, public_access = false \
         WHERE id = ANY($1) AND deleted = false",
    )
    .bind(&body.ids)
    .execute(&state.pg_pool)
    .await;
    match r {
        Ok(r) => respond(200, "Files deleted", vec![], json!({ "deleted": r.rows_affected() })),
        Err(_) => respond(500, "Failed to delete files", vec![], json!({})),
    }
}

#[derive(Deserialize)]
pub struct LinksInput {
    ids: Vec<String>,
}

/// POST /admin/files/disable-links: the bulk form of disable-link.
async fn disable_links(State(state): State<crate::AppState>, Json(body): Json<LinksInput>) -> impl IntoResponse {
    if body.ids.is_empty() || body.ids.len() > 500 {
        return respond(400, "Send between 1 and 500 file ids", vec![], json!({}));
    }
    let r = sqlx::query(
        "UPDATE files SET share_type = 'off', share_token = NULL, public_access = false \
         WHERE id = ANY($1) AND deleted = false AND (share_token IS NOT NULL OR public_access)",
    )
    .bind(&body.ids)
    .execute(&state.pg_pool)
    .await;
    match r {
        Ok(r) => respond(200, "Links disabled", vec![], json!({ "disabled": r.rows_affected() })),
        Err(_) => respond(500, "Failed to disable links", vec![], json!({})),
    }
}

#[derive(Deserialize)]
pub struct ReportInput {
    /// actioned (taken down) or dismissed
    status: String,
}

/// POST /admin/files/{id}/reports: close every open report against a file.
/// "actioned" also turns its link off, so a takedown is one call.
async fn resolve_reports(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
    Json(body): Json<ReportInput>,
) -> impl IntoResponse {
    if body.status != "actioned" && body.status != "dismissed" {
        return respond(400, "Status must be actioned or dismissed", vec![], json!({}));
    }
    let mut tx = match state.pg_pool.begin().await {
        Ok(t) => t,
        Err(_) => return respond(500, "Database error", vec![], json!({})),
    };
    let token = sqlx::query_scalar::<_, Option<String>>("SELECT share_token FROM files WHERE id = $1")
        .bind(&id)
        .fetch_optional(&mut *tx)
        .await;
    let token = match token {
        Ok(Some(t)) => t,
        Ok(None) => return respond(404, "File not found", vec![], json!({})),
        Err(_) => return respond(500, "Database error", vec![], json!({})),
    };
    let closed = sqlx::query(
        "UPDATE abuse_reports SET status = $1 WHERE status = 'open' AND (share_token = $2 OR share_token = $3)",
    )
    .bind(&body.status)
    .bind(&id)
    .bind(&token)
    .execute(&mut *tx)
    .await;
    if body.status == "actioned" {
        let off = sqlx::query("UPDATE files SET share_type = 'off', share_token = NULL, public_access = false WHERE id = $1")
            .bind(&id)
            .execute(&mut *tx)
            .await;
        if off.is_err() {
            return respond(500, "Failed to take the file down", vec![], json!({}));
        }
    }
    match closed {
        Ok(c) => {
            if tx.commit().await.is_err() {
                return respond(500, "Failed to close the reports", vec![], json!({}));
            }
            respond(200, "Reports closed", vec![], json!({ "closed": c.rows_affected() }))
        }
        Err(_) => respond(500, "Failed to close the reports", vec![], json!({})),
    }
}

/// GET /admin/files/{id}/download: reassemble the file from its chunks and
/// stream it as an attachment. Only for UNENCRYPTED files (E2E-encrypted files
/// are stored as ciphertext the server can't decrypt). Size-capped to keep the
/// admin box from OOMing on huge files.
async fn download_file(
    State(state): State<crate::AppState>,
    Path(id): Path<String>,
) -> Response {
    // Sanity ceiling only. The response is streamed chunk-by-chunk below, so peak
    // memory is one chunk regardless of file size and there is no reassembly
    // buffer to OOM the box. This guards an absurd/corrupt size row, not normal
    // large files (which used to fail at 200 MB with "File too large").
    const MAX_BYTES: i64 = 20 * 1024 * 1024 * 1024; // 20 GiB

    let row = sqlx::query_as::<_, (Option<String>, String, String, i64, bool)>(
        "SELECT user_id, name, mime, size, encrypted FROM files WHERE id = $1 AND deleted = false",
    )
    .bind(&id)
    .fetch_optional(&state.pg_pool)
    .await;

    let (user_id, name, mime, size, encrypted) = match row {
        Ok(Some(r)) => r,
        Ok(None) => return respond(404, "File not found", vec![], json!({})).into_response(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    if encrypted {
        return respond(
            400,
            "File is encrypted",
            vec!["This file is end-to-end encrypted; the server cannot produce a decrypted download.".to_string()],
            json!({}),
        )
        .into_response();
    }
    if size > MAX_BYTES {
        return respond(
            413,
            "File too large",
            vec!["This file exceeds the admin download ceiling.".to_string()],
            json!({}),
        )
        .into_response();
    }

    // Owned files live in sanctum; anonymous files in shadow.
    let bucket = if user_id.is_some() { "sanctum" } else { "shadow" }.to_string();

    let chunk_rows = sqlx::query_as::<_, (String,)>(
        "SELECT id FROM chunks WHERE file_id = $1 ORDER BY chunk_index ASC",
    )
    .bind(&id)
    .fetch_all(&state.pg_pool)
    .await;

    let chunk_ids: Vec<String> = match chunk_rows {
        Ok(c) => c.into_iter().map(|(cid,)| cid).collect(),
        Err(_e) => return respond(500, "Database error", vec![], json!({})).into_response(),
    };

    // Stream chunks straight to the client, fetching one at a time (`.then` is
    // sequential), so peak memory is a single chunk no matter how large the file.
    let body_stream = stream::iter(chunk_ids).then(move |cid| {
        let state = state.clone();
        let bucket = bucket.clone();
        async move {
            state
                .r2
                .get_object(&bucket, &cid)
                .await
                .map(Bytes::from)
                .map_err(|_e| std::io::Error::new(std::io::ErrorKind::Other, "storage error"))
        }
    });

    let disposition = format!("attachment; filename=\"{}\"", name.replace('"', ""));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, size.to_string())
        .body(Body::from_stream(body_stream))
        .unwrap()
}

pub fn router() -> Router<crate::AppState> {
    Router::new()
        .route("/", get(list_files))
        .route("/summary", get(summary))
        .route("/delete", post(delete_files))
        .route("/disable-links", post(disable_links))
        .route("/{id}/download", get(download_file))
        .route("/{id}/disable-link", post(disable_link))
        .route("/{id}/reports", post(resolve_reports))
}
