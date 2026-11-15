//! What an upload may declare, and how a stored chunk is checked against it.
//!
//! Quota admission runs on the declared `file_size` because nothing is stored
//! yet. That is only sound if the declaration is what actually lands in R2: the
//! chunk sizes must add up to it, each PUT URL must be signed for exactly its
//! chunk's length, and a chunk is only counted once HeadObject has measured it
//! at that length. Before this, a client could declare 1 byte and PUT 5 GiB per
//! chunk, or declare a negative chunk and mark it complete without uploading,
//! which drove the account's counted usage negative.

use crate::libs::r2::R2;

/// The largest chunk any client sends. packages/client and the API docs use
/// 100 MB chunks; the web app uses 16 MiB.
pub const MAX_CHUNK_BYTES: i64 = 100 * 1024 * 1024;

/// Bounds the chunk count: a file may be split no finer than this on average.
/// It is R2's multipart part minimum, and every shipped client chunks coarser.
/// Each chunk is a presigned PUT and, if abandoned, a delete for the orphan
/// sweep, so thousands of one-byte chunks are refused.
const MIN_AVERAGE_CHUNK_BYTES: i64 = 5 * 1024 * 1024;

/// XChaCha20-Poly1305 appends a 16-byte tag to each encrypted chunk. Declared
/// sizes are plaintext lengths, so this is the difference on the wire.
pub const ENCRYPTION_TAG_BYTES: i64 = 16;

/// The exact byte length a chunk's PUT must carry.
pub fn stored_len(declared: i64, encrypted: bool) -> i64 {
    if encrypted { declared.saturating_add(ENCRYPTION_TAG_BYTES) } else { declared }
}

/// The bytes an upload will store once every chunk lands: what quota admission
/// adds to the account's usage.
pub fn stored_total(chunk_sizes: &[i64], encrypted: bool) -> i64 {
    chunk_sizes.iter().fold(0i64, |sum, &size| sum.saturating_add(stored_len(size, encrypted)))
}

/// Check a declared upload before any URL is minted. An empty file may have no
/// chunks (the drive page) or one empty chunk (the drop page and the client
/// library).
pub fn check_declared(file_size: i64, chunk_sizes: &[i64]) -> Result<(), &'static str> {
    if file_size < 0 {
        return Err("File size cannot be negative.");
    }
    let max_chunks =
        (file_size / MIN_AVERAGE_CHUNK_BYTES + i64::from(file_size % MIN_AVERAGE_CHUNK_BYTES != 0)).max(1);
    if chunk_sizes.len() as i64 > max_chunks {
        return Err("Too many chunks for this file size.");
    }
    let mut total: i64 = 0;
    for &size in chunk_sizes {
        if !(0..=MAX_CHUNK_BYTES).contains(&size) {
            return Err("Each chunk must be between 0 and 100 MiB.");
        }
        total = total.saturating_add(size);
    }
    if total != file_size {
        return Err("Chunk sizes must add up to the file size.");
    }
    Ok(())
}

/// The length a chunk's PUT URL was signed for, from its declared size and
/// whether its file is encrypted. `None` if the chunk does not exist.
pub async fn chunk_expected_len(pool: &sqlx::PgPool, chunk_id: &str) -> Result<Option<i64>, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64, bool)>(
        "SELECT c.size, f.encrypted FROM chunks c JOIN files f ON f.id = c.file_id WHERE c.id = $1",
    )
    .bind(chunk_id)
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|(size, encrypted)| stored_len(size, encrypted)))
}

pub enum Measured {
    /// Stored at exactly the expected length.
    Stored(i64),
    /// An object is there but at another length.
    WrongSize,
    /// HeadObject failed: not uploaded yet, or storage did not answer.
    Unconfirmed,
}

/// Measure a chunk's object and compare it with the length its URL was signed
/// for. The declared size is never used in place of a measurement.
pub async fn measure_chunk(r2: &R2, storage: &str, key: &str, expected: i64) -> Measured {
    match r2.object_size(storage, key).await {
        Ok(size) if size == expected => Measured::Stored(size),
        Ok(_) => Measured::WrongSize,
        Err(_) => Measured::Unconfirmed,
    }
}

/// Give back the request-link slot of each unfinished upload among `file_ids`
/// and untie it from its request, so it can no longer finish through the link.
/// Call it before deleting files, since afterwards nothing says which of them
/// were unfinished. A finished upload keeps its slot even once deleted: the
/// cap counts files received, not files kept.
pub async fn release_request_slots<'e, E>(ex: E, file_ids: &[String]) -> Result<(), sqlx::Error>
where
    E: sqlx::PgExecutor<'e>,
{
    if file_ids.is_empty() {
        return Ok(());
    }
    sqlx::query(
        "WITH slots AS ( \
             DELETE FROM request_uploads ru USING files f \
             WHERE ru.file_id = f.id AND f.id = ANY($1) AND f.uploaded_chunks < f.total_chunks \
             RETURNING ru.request_id \
         ), per AS (SELECT request_id, COUNT(*)::int AS n FROM slots GROUP BY request_id) \
         UPDATE file_requests r SET upload_count = GREATEST(r.upload_count - per.n, 0) \
         FROM per WHERE r.id = per.request_id",
    )
    .bind(file_ids)
    .execute(ex)
    .await
    .map(|_| ())
}

/// release_request_slots for every file in a folder and the folders below it.
pub async fn release_request_slots_in_folder(pool: &sqlx::PgPool, folder_id: &str) -> Result<(), sqlx::Error> {
    // UNION, not UNION ALL, so a parent_id cycle ends the walk.
    let ids: Vec<String> = sqlx::query_scalar(
        "WITH RECURSIVE subtree AS ( \
             SELECT id FROM folders WHERE id = $1 \
             UNION \
             SELECT f.id FROM folders f JOIN subtree s ON f.parent_id = s.id \
         ) \
         SELECT id FROM files WHERE folder_id IN (SELECT id FROM subtree)",
    )
    .bind(folder_id)
    .fetch_all(pool)
    .await?;
    release_request_slots(pool, &ids).await
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIB: i64 = 1024 * 1024;

    #[test]
    fn the_shapes_every_client_sends_pass() {
        // web app: 16 MiB chunks; client library: 100 MB chunks
        assert!(check_declared(40 * MIB, &[16 * MIB, 16 * MIB, 8 * MIB]).is_ok());
        assert!(check_declared(250 * MIB, &[100 * MIB, 100 * MIB, 50 * MIB]).is_ok());
        assert!(check_declared(0, &[]).is_ok());
        assert!(check_declared(0, &[0]).is_ok());
        assert!(check_declared(5, &[5]).is_ok());
    }

    #[test]
    fn dishonest_shapes_fail() {
        assert!(check_declared(-1, &[]).is_err());
        assert!(check_declared(0, &[i64::MIN / 2]).is_err());
        assert!(check_declared(0, &[-5, 5]).is_err());
        assert!(check_declared(1, &[50 * MIB]).is_err());
        assert!(check_declared(200 * MIB, &[200 * MIB]).is_err());
        assert!(check_declared(5000, &vec![1; 5000]).is_err());
        assert!(check_declared(0, &[0, 0]).is_err());
        assert!(check_declared(i64::MAX, &[MAX_CHUNK_BYTES]).is_err());
    }

    #[test]
    fn encrypted_chunks_carry_the_tag() {
        assert_eq!(stored_len(16 * MIB, true), 16 * MIB + 16);
        assert_eq!(stored_len(16 * MIB, false), 16 * MIB);
        assert_eq!(stored_len(0, true), 16);
        assert_eq!(stored_total(&[16 * MIB, 5], true), 16 * MIB + 5 + 32);
        assert_eq!(stored_total(&[], true), 0);
    }
}
