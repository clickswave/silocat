//! Shared rules for reading a file or folder by its id.
//!
//! Two questions come up on every by-id read path (`fetch_resource`,
//! `fetch_chunks`, and the folder listings), and both were being answered by
//! copies of the same expression that had already drifted apart once.

use chrono::{DateTime, Utc};

/// Does an active share protection apply, meaning this resource must NOT be
/// served to a non-owner who supplies only its id?
///
/// `public_access` says the owner published the thing; this says the publication
/// carries a control that only the token path can actually enforce. A password
/// is checked on `/file/public/share/authorize`, a one-time link's counter is
/// decremented there, and an expiry has simply run out. Serving any of those by
/// id would hand out the content with the control skipped, which is how a
/// password on a drop used to be worth nothing.
///
/// A deleted resource is protected in the same sense: gone.
pub fn share_protected(
    share_password_hash: Option<&str>,
    share_type: Option<&str>,
    share_expires_at: Option<DateTime<Utc>>,
    deleted: bool,
) -> bool {
    share_password_hash.is_some()
        || share_type == Some("once")
        || share_expires_at.map_or(false, |e| e <= Utc::now())
        || deleted
}

/// How deep a folder tree is walked, and how many files one folder share can
/// list. Both are here to bound the work a single unauthenticated request can
/// ask for, and the depth cap doubles as the cycle guard: `parent_id` is a plain
/// self-reference with no constraint stopping A from parenting B and B from
/// parenting A, which would otherwise be an unbounded recursion.
const MAX_DEPTH: i32 = 32;
const MAX_FILES: i64 = 5000;

/// A folder in a subtree, with its path relative to the subtree's root.
/// The root itself comes back with an empty path.
pub struct SubtreeFolder {
    pub id: String,
    pub path: String,
}

/// Every folder at or under `root_id`, deleted ones excluded.
///
/// A folder share is a share of the folder's contents, so the unit is the whole
/// subtree. Listing only direct children (which is what every folder path did
/// before) silently dropped everything in a subdirectory: upload a folder with
/// any structure at all and the recipient saw a fraction of it, with no sign
/// that the rest existed.
pub async fn folder_subtree(
    pool: &sqlx::PgPool,
    root_id: &str,
) -> Result<Vec<SubtreeFolder>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE tree AS (
            SELECT id, ''::text AS rel, 0 AS depth
              FROM folders
             WHERE id = $1
            UNION ALL
            SELECT f.id,
                   CASE WHEN t.rel = '' THEN f.name ELSE t.rel || '/' || f.name END,
                   t.depth + 1
              FROM folders f
              JOIN tree t ON f.parent_id = t.id
             WHERE f.deleted = false
               AND t.depth < $2
        )
        SELECT id AS "id!", rel AS "rel!" FROM tree
        "#,
        root_id,
        MAX_DEPTH
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| SubtreeFolder { id: r.id, path: r.rel })
        .collect())
}

/// The files in a subtree, paired with the relative path of the folder each one
/// sits in, ordered so a listing reads like a directory walk.
pub async fn subtree_files(
    pool: &sqlx::PgPool,
    folders: &[SubtreeFolder],
) -> Result<Vec<(crate::models::File, String)>, sqlx::Error> {
    let ids: Vec<String> = folders.iter().map(|f| f.id.clone()).collect();
    let files = sqlx::query_as!(
        crate::models::File,
        "SELECT * FROM files WHERE folder_id = ANY($1) AND deleted = false \
         ORDER BY name LIMIT $2",
        &ids,
        MAX_FILES
    )
    .fetch_all(pool)
    .await?;

    let mut out: Vec<(crate::models::File, String)> = files
        .into_iter()
        .map(|f| {
            let path = f
                .folder_id
                .as_ref()
                .and_then(|fid| folders.iter().find(|sf| &sf.id == fid))
                .map(|sf| sf.path.clone())
                .unwrap_or_default();
            (f, path)
        })
        .collect();
    out.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.name.cmp(&b.0.name)));
    Ok(out)
}

/// Is `file_id` a live file somewhere at or under `root_id`? Returns its id and
/// `user_id` (which decides the storage bucket) when it is.
///
/// The token path used to answer this with `folder_id = root_id`, so a file one
/// directory down was listed by the share and then refused at download.
pub async fn file_in_subtree(
    pool: &sqlx::PgPool,
    root_id: &str,
    file_id: &str,
) -> Result<Option<(String, Option<String>)>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        WITH RECURSIVE tree AS (
            SELECT id, 0 AS depth
              FROM folders
             WHERE id = $1
            UNION ALL
            SELECT f.id, t.depth + 1
              FROM folders f
              JOIN tree t ON f.parent_id = t.id
             WHERE f.deleted = false
               AND t.depth < $3
        )
        SELECT fi.id AS "id!", fi.user_id
          FROM files fi
         WHERE fi.id = $2
           AND fi.deleted = false
           AND fi.folder_id IN (SELECT id FROM tree)
        "#,
        root_id,
        file_id,
        MAX_DEPTH
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|r| (r.id, r.user_id)))
}

/// Serialize a file for a share listing: the usual file JSON plus the `path` of
/// the folder it sits in, relative to the shared root. Clients use it to render
/// the tree and to rebuild the directory structure inside a downloaded zip.
pub fn file_with_path(file: &crate::models::File, path: &str) -> serde_json::Value {
    let mut value = serde_json::to_value(file).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(object) = value.as_object_mut() {
        object.insert("path".to_string(), serde_json::json!(path));
    }
    value
}

/// What a folder and everything above it say about reading by id.
pub struct FolderAccess {
    /// This folder, or one above it, is published.
    pub published: bool,
    /// This folder, or one above it, carries a control that only the token path
    /// can enforce, so nothing under it may be served by id.
    pub protected: bool,
}

impl FolderAccess {
    /// Fail closed: an unknown or unreadable chain grants nothing.
    fn closed() -> Self {
        FolderAccess { published: false, protected: true }
    }

    pub fn grants_public_read(&self) -> bool {
        self.published && !self.protected
    }
}

/// Walk from a folder up to the root of its tree.
///
/// Both answers have to come from the whole chain, not from the folder itself.
/// Publishing a folder publishes its subtree, so a subfolder opens by its own id
/// as well. And a password or an expiry on a folder has to cover everything
/// under it, otherwise the protection is bypassed by asking for a subfolder id
/// or a file id one level down, which the listing hands out.
pub async fn folder_access(pool: &sqlx::PgPool, folder_id: &str) -> FolderAccess {
    let rows = sqlx::query!(
        r#"
        WITH RECURSIVE chain AS (
            SELECT id, parent_id, public_access, share_type, share_expires_at,
                   share_password_hash, deleted, 0 AS depth
              FROM folders
             WHERE id = $1
            UNION ALL
            SELECT f.id, f.parent_id, f.public_access, f.share_type, f.share_expires_at,
                   f.share_password_hash, f.deleted, c.depth + 1
              FROM folders f
              JOIN chain c ON f.id = c.parent_id
             WHERE c.depth < $2
        )
        SELECT public_access AS "public_access!", share_type, share_expires_at,
               share_password_hash, deleted AS "deleted!"
          FROM chain
        "#,
        folder_id,
        MAX_DEPTH
    )
    .fetch_all(pool)
    .await;

    let Ok(rows) = rows else { return FolderAccess::closed() };
    if rows.is_empty() {
        return FolderAccess::closed();
    }

    FolderAccess {
        published: rows.iter().any(|r| r.public_access),
        protected: rows.iter().any(|r| {
            share_protected(
                r.share_password_hash.as_deref(),
                r.share_type.as_deref(),
                r.share_expires_at,
                r.deleted,
            )
        }),
    }
}
