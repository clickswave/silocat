//! The owner of an address claiming an account nobody had verified.
//!
//! Email signup creates the account before anyone shows they control the
//! inbox, and gives the registrant the account's API key at once. So an
//! unverified row may have been set up by someone other than the owner of the
//! address. When the owner later proves the address, by Google sign-in or with
//! a reset code sent to that inbox, everything the registrant set is untrusted:
//!
//!   - the API key is replaced, and files and folders stamped with it move to
//!     the new one. The key is also what a web session carries, so every
//!     session and integration the registrant holds stops working;
//!   - a pending email change is dropped;
//!   - on Google sign-in the password is cleared and the username and bio are
//!     reset, since the owner chose none of them. A password reset sets its
//!     own password, and the owner can see and keep or change the rest.
//!
//! A verified account was set up by someone who proved the address, so it is
//! left as it is.

use crate::libs::apikey::NewKey;
use sqlx::PgConnection;

/// Google has verified that the caller owns this account's address. `key` is
/// used only when the account turns out to be unverified.
pub(super) async fn google_sign_in(
    conn: &mut PgConnection,
    user_id: &str,
    username_base: &str,
    key: &NewKey,
) -> Result<(), sqlx::Error> {
    if !lock_unverified(conn, user_id).await? {
        return Ok(());
    }
    take_over(conn, user_id, key).await?;

    let username = free_username(conn, username_base).await?;
    sqlx::query(
        "UPDATE users SET email_verified = true, otp = '', password_hash = '', \
         username = $1, username_change_count = 0, username_change_window_start = NULL, \
         bio = NULL WHERE id = $2",
    )
    .bind(&username)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// The caller entered a reset code sent to this account's address and chose
/// `new_hash` as the password. `key` is used only when the account turns out
/// to be unverified.
pub(super) async fn password_reset(
    conn: &mut PgConnection,
    user_id: &str,
    new_hash: &str,
    key: &NewKey,
) -> Result<(), sqlx::Error> {
    if lock_unverified(conn, user_id).await? {
        take_over(conn, user_id, key).await?;
    }
    sqlx::query("UPDATE users SET password_hash = $1, otp = '', email_verified = true WHERE id = $2")
        .bind(new_hash)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// Lock the row and report whether it is still unverified. Two claims racing
/// on one account serialize here, and the second sees it verified.
async fn lock_unverified(conn: &mut PgConnection, user_id: &str) -> Result<bool, sqlx::Error> {
    let verified: Option<bool> =
        sqlx::query_scalar("SELECT email_verified FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(verified == Some(false))
}

/// What every claim of an unverified account revokes.
async fn take_over(conn: &mut PgConnection, user_id: &str, key: &NewKey) -> Result<(), sqlx::Error> {
    super::rotate_api_key::replace_key(conn, user_id, &key.blind_index, &key.encrypted).await?;
    sqlx::query("UPDATE users SET pending_email = NULL, pending_email_otp = NULL WHERE id = $1")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// `base` plus four digits, the shape a first Google sign-in gets, picked so it
/// is not already taken.
async fn free_username(conn: &mut PgConnection, base: &str) -> Result<String, sqlx::Error> {
    let mut candidate = String::new();
    for _ in 0..8 {
        candidate = format!("{}{}", base, crate::libs::rng::number(4));
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(&candidate)
            .fetch_one(&mut *conn)
            .await?;
        if !taken {
            break;
        }
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    //! These run against a Postgres that has the api_switch migrations applied,
    //! named by SILOCAT_TEST_DATABASE_URL, and skip when it is unset. Each test
    //! works inside one transaction and never commits it.

    use super::*;
    use crate::libs::{apikey, rng};
    use sqlx::{Connection, Row};

    async fn connect() -> Option<PgConnection> {
        let url = std::env::var("SILOCAT_TEST_DATABASE_URL").ok()?;
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        Some(PgConnection::connect(&url).await.expect("connect to SILOCAT_TEST_DATABASE_URL"))
    }

    struct Seeded {
        id: String,
        old_raw: String,
        old_index: String,
        username: String,
        folder_id: String,
    }

    /// An account as email signup leaves it, with what a registrant could have
    /// set on top: a bio, a pending email change and a folder stamped with the
    /// key.
    async fn seed(conn: &mut PgConnection, verified: bool) -> Seeded {
        let id = rng::uuid();
        let key = apikey::mint().expect("test key");
        let username = format!("squatter{}", rng::string(10)).to_lowercase();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
             api_key_migrated, account_type, email_verified, bio, pending_email, pending_email_otp) \
             VALUES ($1, $2, $3, 'registrant-hash', '424242', $4, $5, TRUE, 'personal', $6, \
             'set by the registrant', 'registrant@example.net', '135790')",
        )
        .bind(&id)
        .bind(&username)
        .bind(format!("{}@example.com", id))
        .bind(&key.blind_index)
        .bind(&key.encrypted)
        .bind(verified)
        .execute(&mut *conn)
        .await
        .expect("seed user");
        let folder_id: String = sqlx::query_scalar(
            "INSERT INTO folders (name, user_id, owner_api_key) VALUES ('f', $1, $2) RETURNING id::text",
        )
        .bind(&id)
        .bind(&key.blind_index)
        .fetch_one(&mut *conn)
        .await
        .expect("seed folder");
        Seeded { id, old_raw: key.raw, old_index: key.blind_index, username, folder_id }
    }

    async fn user_row(conn: &mut PgConnection, id: &str) -> sqlx::postgres::PgRow {
        sqlx::query(
            "SELECT api_key, api_key_enc, password_hash, username, bio, email_verified, otp, \
             pending_email, pending_email_otp FROM users WHERE id = $1",
        )
        .bind(id)
        .fetch_one(&mut *conn)
        .await
        .unwrap()
    }

    async fn folder_key(conn: &mut PgConnection, folder_id: &str) -> String {
        sqlx::query_scalar("SELECT owner_api_key FROM folders WHERE id::text = $1")
            .bind(folder_id)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    async fn key_resolves(conn: &mut PgConnection, index: &str) -> bool {
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE api_key = $1)")
            .bind(index)
            .fetch_one(&mut *conn)
            .await
            .unwrap()
    }

    /// The registrant's key, session and password stop working, and the owner
    /// is left holding a key nobody else has seen.
    #[tokio::test]
    async fn google_sign_in_takes_an_unverified_account_from_its_registrant() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        let key = apikey::mint().unwrap();

        google_sign_in(&mut tx, &s.id, "owner", &key).await.unwrap();

        let row = user_row(&mut tx, &s.id).await;
        assert!(row.get::<bool, _>("email_verified"));
        assert!(!key_resolves(&mut tx, &s.old_index).await, "the registrant's key still opens the account");
        assert_eq!(row.get::<String, _>("api_key"), key.blind_index);
        let shown = apikey::decrypt(&row.get::<Option<String>, _>("api_key_enc").unwrap()).unwrap();
        assert_eq!(shown, key.raw);
        assert_ne!(shown, s.old_raw);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, key.blind_index, "the folder stayed on the old key");
        assert_eq!(row.get::<String, _>("password_hash"), "", "the registrant's password still signs in");
        let username: String = row.get("username");
        assert_ne!(username, s.username);
        assert!(username.starts_with("owner") && username.len() == "owner".len() + 4);
        assert_eq!(row.get::<Option<String>, _>("bio"), None);
        assert_eq!(row.get::<Option<String>, _>("pending_email"), None);
        assert_eq!(row.get::<Option<String>, _>("pending_email_otp"), None);
        assert_eq!(row.get::<String, _>("otp"), "");
    }

    #[tokio::test]
    async fn google_sign_in_leaves_a_verified_account_alone() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, true).await;
        let key = apikey::mint().unwrap();

        google_sign_in(&mut tx, &s.id, "owner", &key).await.unwrap();

        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("api_key"), s.old_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, s.old_index);
        assert_eq!(row.get::<String, _>("password_hash"), "registrant-hash");
        assert_eq!(row.get::<String, _>("username"), s.username);
        assert_eq!(row.get::<Option<String>, _>("bio").as_deref(), Some("set by the registrant"));
        assert_eq!(row.get::<Option<String>, _>("pending_email").as_deref(), Some("registrant@example.net"));
    }

    #[tokio::test]
    async fn password_reset_takes_an_unverified_account_from_its_registrant() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        let key = apikey::mint().unwrap();

        password_reset(&mut tx, &s.id, "owner-hash", &key).await.unwrap();

        let row = user_row(&mut tx, &s.id).await;
        assert!(row.get::<bool, _>("email_verified"));
        assert_eq!(row.get::<String, _>("password_hash"), "owner-hash");
        assert!(!key_resolves(&mut tx, &s.old_index).await, "the registrant's key still opens the account");
        assert_eq!(row.get::<String, _>("api_key"), key.blind_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, key.blind_index, "the folder stayed on the old key");
        assert_eq!(row.get::<Option<String>, _>("pending_email"), None);
        assert_eq!(row.get::<Option<String>, _>("pending_email_otp"), None);
        assert_eq!(row.get::<String, _>("otp"), "");
    }

    /// An owner resetting a forgotten password keeps the key their
    /// integrations use.
    #[tokio::test]
    async fn password_reset_keeps_the_key_of_a_verified_account() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, true).await;
        let key = apikey::mint().unwrap();

        password_reset(&mut tx, &s.id, "owner-hash", &key).await.unwrap();

        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("password_hash"), "owner-hash");
        assert_eq!(row.get::<String, _>("api_key"), s.old_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, s.old_index);
        assert_eq!(row.get::<String, _>("username"), s.username);
    }
}
