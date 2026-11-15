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
//!   - every share link on the account's files and folders is turned off and
//!     its token dropped, and every file request link is deactivated. An
//!     unverified account can make folders and publish them, so a folder the
//!     registrant shared would otherwise serve whatever the owner later puts
//!     in it to anyone with the link;
//!   - a pending email change is dropped;
//!   - the username, bio and avatar are reset, since the owner chose none of
//!     them;
//!   - on Google sign-in the password is cleared. A password reset sets its
//!     own.
//!
//! Requests the registrant had in flight when the claim committed are refused
//! where they would hand out a credential or a link again: see
//! libs::key_guard.
//!
//! A verified account was set up by someone who proved the address, so it is
//! left as it is.

use crate::libs::apikey::NewKey;
use sqlx::PgConnection;

/// What a claim found once it held the account row.
#[derive(Debug, PartialEq)]
pub(super) enum Claim {
    /// The account was verified, so nothing was taken from anyone.
    Verified,
    /// The account was unverified and now belongs to the caller.
    TakenOver,
    /// The row no longer carries the address the caller proved (its email
    /// changed after it was looked up), or it is gone. Nothing was written,
    /// and the caller must not be signed in to it.
    Moved,
}

/// Google has verified that the caller owns `email`, the address this account
/// was found by. `key` is used only when the account turns out to be
/// unverified, and `picture` is the Google profile picture it then gets, as a
/// first Google sign-in would.
pub(super) async fn google_sign_in(
    conn: &mut PgConnection,
    user_id: &str,
    email: &str,
    username_base: &str,
    picture: Option<&str>,
    key: &NewKey,
) -> Result<Claim, sqlx::Error> {
    let found = lock(conn, user_id, email).await?;
    if found != Claim::TakenOver {
        return Ok(found);
    }
    take_over(conn, user_id, username_base, picture, key).await?;
    sqlx::query("UPDATE users SET email_verified = true, otp = '', password_hash = '' WHERE id = $1")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(Claim::TakenOver)
}

/// The caller entered a reset code sent to `email`, the address this account
/// was found by, and chose `new_hash` as the password. `key` is used only when
/// the account turns out to be unverified.
pub(super) async fn password_reset(
    conn: &mut PgConnection,
    user_id: &str,
    email: &str,
    new_hash: &str,
    key: &NewKey,
) -> Result<Claim, sqlx::Error> {
    let found = lock(conn, user_id, email).await?;
    match found {
        Claim::Moved => return Ok(found),
        Claim::TakenOver => take_over(conn, user_id, &email_base(email), None, key).await?,
        Claim::Verified => {}
    }
    sqlx::query("UPDATE users SET password_hash = $1, otp = '', email_verified = true WHERE id = $2")
        .bind(new_hash)
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(found)
}

/// Lock the row and say what a claim of it would do: `TakenOver` while it is
/// unverified and still carries `email`. Two claims racing on one account
/// serialize here, and the second sees it verified.
async fn lock(conn: &mut PgConnection, user_id: &str, email: &str) -> Result<Claim, sqlx::Error> {
    let row: Option<(bool, String)> =
        sqlx::query_as("SELECT email_verified, email FROM users WHERE id = $1 FOR UPDATE")
            .bind(user_id)
            .fetch_optional(&mut *conn)
            .await?;
    Ok(match row {
        Some((_, current)) if current != email => Claim::Moved,
        Some((true, _)) => Claim::Verified,
        Some((false, _)) => Claim::TakenOver,
        None => Claim::Moved,
    })
}

/// What every claim of an unverified account revokes or resets.
async fn take_over(
    conn: &mut PgConnection,
    user_id: &str,
    username_base: &str,
    picture: Option<&str>,
    key: &NewKey,
) -> Result<(), sqlx::Error> {
    super::rotate_api_key::replace_key(conn, user_id, &key.blind_index, &key.encrypted).await?;

    let username = free_username(conn, username_base).await?;
    sqlx::query(
        "UPDATE users SET pending_email = NULL, pending_email_otp = NULL, username = $1, \
         username_change_count = 0, username_change_window_start = NULL, bio = NULL, \
         profile_image = $2 WHERE id = $3",
    )
    .bind(&username)
    .bind(picture)
    .bind(user_id)
    .execute(&mut *conn)
    .await?;

    // Same columns toggle_share writes when sharing is turned off, plus the
    // expiry and password, so nothing of the old link comes back if the owner
    // shares the item again.
    for table in ["folders", "files"] {
        sqlx::query(&format!(
            "UPDATE {table} SET share_type = 'off', share_token = NULL, public_access = false, \
             share_expires_at = NULL, share_password_hash = NULL WHERE user_id = $1"
        ))
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    }
    sqlx::query("UPDATE file_requests SET active = false WHERE owner_user_id = $1")
        .bind(user_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

/// The username stem for an account claimed through a reset code: the
/// address's local part.
fn email_base(email: &str) -> String {
    email.split('@').next().unwrap_or_default().to_string()
}

/// A username that is free right now: `base` (cut down to lowercase letters
/// and digits, at most 18 of them) plus four digits, the shape a first Google
/// sign-in gets. If those keep colliding the suffix grows to eight digits and
/// then twelve, so someone who registers most of the four-digit names cannot
/// make the claim fail.
pub(super) async fn free_username(conn: &mut PgConnection, base: &str) -> Result<String, sqlx::Error> {
    let mut stem: String = base
        .to_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(18)
        .collect();
    if stem.is_empty() {
        stem = "user".to_string();
    }
    for attempt in 0..24 {
        let digits = match attempt {
            0..8 => 4,
            8..16 => 8,
            _ => 12,
        };
        let candidate = format!("{}{}", stem, crate::libs::rng::number(digits));
        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(&candidate)
            .fetch_one(&mut *conn)
            .await?;
        if !taken {
            return Ok(candidate);
        }
    }
    Err(sqlx::Error::Protocol(format!("no free username for stem {stem}")))
}

#[cfg(test)]
mod tests {
    //! These run against a Postgres that has the api_switch migrations applied,
    //! named by SILOCAT_TEST_DATABASE_URL, and skip when it is unset. Each test
    //! works inside one transaction and never commits it, except the one that
    //! needs two connections, which deletes what it made.

    use super::*;
    use crate::libs::{apikey, key_guard, rng};
    use sqlx::{Connection, Row};

    async fn connect() -> Option<PgConnection> {
        let url = std::env::var("SILOCAT_TEST_DATABASE_URL").ok()?;
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        Some(PgConnection::connect(&url).await.expect("connect to SILOCAT_TEST_DATABASE_URL"))
    }

    struct Seeded {
        id: String,
        email: String,
        old_raw: String,
        old_index: String,
        username: String,
        folder_id: String,
        folder_token: String,
        file_id: String,
        request_id: String,
    }

    /// An account as email signup leaves it, with what a registrant could have
    /// set on top: a bio, an avatar, a pending email change, used-up username
    /// changes, a folder stamped with the key and published by link, a shared
    /// file and an open file request.
    async fn seed(conn: &mut PgConnection, verified: bool) -> Seeded {
        let id = rng::uuid();
        let email = format!("{}@example.com", id);
        let key = apikey::mint().expect("test key");
        let username = format!("squatter{}", rng::string(10)).to_lowercase();
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, api_key_enc, \
             api_key_migrated, account_type, email_verified, bio, pending_email, pending_email_otp, \
             profile_image, username_change_count, username_change_window_start) \
             VALUES ($1, $2, $3, 'registrant-hash', '424242', $4, $5, TRUE, 'personal', $6, \
             'set by the registrant', 'registrant@example.net', '135790', \
             'https://registrant.example/avatar.png', 3, NOW())",
        )
        .bind(&id)
        .bind(&username)
        .bind(&email)
        .bind(&key.blind_index)
        .bind(&key.encrypted)
        .bind(verified)
        .execute(&mut *conn)
        .await
        .expect("seed user");
        let folder_token = rng::string(32);
        let folder_id: String = sqlx::query_scalar(
            "INSERT INTO folders (name, user_id, owner_api_key, share_type, share_token, public_access, \
             share_password_hash, share_expires_at) \
             VALUES ('Photos', $1, $2, 'public', $3, true, 'link-hash', NOW() + INTERVAL '1 day') \
             RETURNING id::text",
        )
        .bind(&id)
        .bind(&key.blind_index)
        .bind(&folder_token)
        .fetch_one(&mut *conn)
        .await
        .expect("seed folder");
        let file_id: String = sqlx::query_scalar(
            "INSERT INTO files (user_id, owner_api_key, name, mime, size, encrypted, total_chunks, \
             sha256_checksum, blake3_checksum, public_access, share_type, share_token) \
             VALUES ($1, $2, 'f.txt', 'text/plain', 1, false, 1, 's', 'b', true, 'once', $3) \
             RETURNING id",
        )
        .bind(&id)
        .bind(&key.blind_index)
        .bind(rng::string(32))
        .fetch_one(&mut *conn)
        .await
        .expect("seed file");
        let request_id: String = sqlx::query_scalar(
            "INSERT INTO file_requests (token, owner_user_id) VALUES ($1, $2) RETURNING id",
        )
        .bind(rng::string(40))
        .bind(&id)
        .fetch_one(&mut *conn)
        .await
        .expect("seed file request");
        Seeded {
            id,
            email,
            old_raw: key.raw,
            old_index: key.blind_index,
            username,
            folder_id,
            folder_token,
            file_id,
            request_id,
        }
    }

    async fn user_row(conn: &mut PgConnection, id: &str) -> sqlx::postgres::PgRow {
        sqlx::query(
            "SELECT api_key, api_key_enc, password_hash, username, bio, email_verified, otp, \
             pending_email, pending_email_otp, profile_image, username_change_count, \
             username_change_window_start IS NULL AS window_clear FROM users WHERE id = $1",
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

    /// The registrant's links: what public share info would serve for the
    /// folder token, whether the folder or file is readable by id, and whether
    /// the request link still takes uploads.
    struct Links {
        folder_by_token: bool,
        folder_public: bool,
        folder_password: bool,
        file_shared: bool,
        request_active: bool,
    }

    async fn links(conn: &mut PgConnection, s: &Seeded) -> Links {
        // The lookup share.rs makes for /public/share/info/{token}.
        let folder_by_token: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM folders WHERE share_token = $1 AND share_type != 'off' AND deleted = false)",
        )
        .bind(&s.folder_token)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        let (folder_public, folder_password): (bool, bool) = sqlx::query_as(
            "SELECT public_access, share_password_hash IS NOT NULL FROM folders WHERE id::text = $1",
        )
        .bind(&s.folder_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        let file_shared: bool = sqlx::query_scalar(
            "SELECT public_access OR share_type <> 'off' OR share_token IS NOT NULL FROM files WHERE id = $1",
        )
        .bind(&s.file_id)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
        let request_active: bool = sqlx::query_scalar("SELECT active FROM file_requests WHERE id = $1")
            .bind(&s.request_id)
            .fetch_one(&mut *conn)
            .await
            .unwrap();
        Links { folder_by_token, folder_public, folder_password, file_shared, request_active }
    }

    fn assert_links_dead(l: &Links) {
        assert!(!l.folder_by_token, "the registrant's folder link still serves the folder");
        assert!(!l.folder_public, "the registrant's folder is still readable by id");
        assert!(!l.folder_password, "the registrant's link password is still on the folder");
        assert!(!l.file_shared, "the registrant's file share is still live");
        assert!(!l.request_active, "the registrant's request link still takes uploads");
    }

    /// The registrant's key, session, password and links stop working, and the
    /// owner is left holding a key nobody else has seen.
    #[tokio::test]
    async fn google_sign_in_takes_an_unverified_account_from_its_registrant() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        let key = apikey::mint().unwrap();

        let claim = google_sign_in(&mut tx, &s.id, &s.email, "owner", Some("https://google.example/me.png"), &key)
            .await
            .unwrap();

        assert_eq!(claim, Claim::TakenOver);
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
        assert_eq!(row.get::<i32, _>("username_change_count"), 0);
        assert!(row.get::<bool, _>("window_clear"));
        assert_eq!(row.get::<Option<String>, _>("bio"), None);
        assert_eq!(row.get::<Option<String>, _>("profile_image").as_deref(), Some("https://google.example/me.png"));
        assert_eq!(row.get::<Option<String>, _>("pending_email"), None);
        assert_eq!(row.get::<Option<String>, _>("pending_email_otp"), None);
        assert_eq!(row.get::<String, _>("otp"), "");
        assert_links_dead(&links(&mut tx, &s).await);
    }

    #[tokio::test]
    async fn google_sign_in_leaves_a_verified_account_alone() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, true).await;
        let key = apikey::mint().unwrap();

        let claim = google_sign_in(&mut tx, &s.id, &s.email, "owner", None, &key).await.unwrap();

        assert_eq!(claim, Claim::Verified);
        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("api_key"), s.old_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, s.old_index);
        assert_eq!(row.get::<String, _>("password_hash"), "registrant-hash");
        assert_eq!(row.get::<String, _>("username"), s.username);
        assert_eq!(row.get::<Option<String>, _>("bio").as_deref(), Some("set by the registrant"));
        assert_eq!(row.get::<Option<String>, _>("pending_email").as_deref(), Some("registrant@example.net"));
        let l = links(&mut tx, &s).await;
        assert!(l.folder_by_token && l.folder_public && l.file_shared && l.request_active);
    }

    #[tokio::test]
    async fn password_reset_takes_an_unverified_account_from_its_registrant() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        let key = apikey::mint().unwrap();

        let claim = password_reset(&mut tx, &s.id, &s.email, "owner-hash", &key).await.unwrap();

        assert_eq!(claim, Claim::TakenOver);
        let row = user_row(&mut tx, &s.id).await;
        assert!(row.get::<bool, _>("email_verified"));
        assert_eq!(row.get::<String, _>("password_hash"), "owner-hash");
        assert!(!key_resolves(&mut tx, &s.old_index).await, "the registrant's key still opens the account");
        assert_eq!(row.get::<String, _>("api_key"), key.blind_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, key.blind_index, "the folder stayed on the old key");
        assert_eq!(row.get::<Option<String>, _>("pending_email"), None);
        assert_eq!(row.get::<Option<String>, _>("pending_email_otp"), None);
        assert_eq!(row.get::<String, _>("otp"), "");
        // The username comes from the address, as the owner chose nothing else.
        let username: String = row.get("username");
        assert_ne!(username, s.username, "the registrant's username stayed on the account");
        let stem: String = s.id.chars().filter(|c| c.is_ascii_alphanumeric()).take(18).collect();
        assert!(username.starts_with(&stem) && username.len() == stem.len() + 4, "{username}");
        assert_eq!(row.get::<i32, _>("username_change_count"), 0);
        assert_eq!(row.get::<Option<String>, _>("bio"), None);
        assert_eq!(row.get::<Option<String>, _>("profile_image"), None);
        assert_links_dead(&links(&mut tx, &s).await);
    }

    /// An owner resetting a forgotten password keeps the key, name and links
    /// they set up.
    #[tokio::test]
    async fn password_reset_keeps_the_key_of_a_verified_account() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, true).await;
        let key = apikey::mint().unwrap();

        let claim = password_reset(&mut tx, &s.id, &s.email, "owner-hash", &key).await.unwrap();

        assert_eq!(claim, Claim::Verified);
        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("password_hash"), "owner-hash");
        assert_eq!(row.get::<String, _>("api_key"), s.old_index);
        assert_eq!(folder_key(&mut tx, &s.folder_id).await, s.old_index);
        assert_eq!(row.get::<String, _>("username"), s.username);
        let l = links(&mut tx, &s).await;
        assert!(l.folder_by_token && l.folder_public && l.file_shared && l.request_active);
    }

    /// The row was found by the owner's address, but by the time it is locked
    /// it carries another one (the registrant confirmed an email change in
    /// between). The caller is not signed in to it and nothing is written.
    #[tokio::test]
    async fn a_claim_of_a_row_whose_address_changed_signs_nobody_in() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        sqlx::query("UPDATE users SET email = $1, email_verified = true WHERE id = $2")
            .bind(format!("registrant-{}@example.net", s.id))
            .bind(&s.id)
            .execute(&mut *tx)
            .await
            .unwrap();
        let key = apikey::mint().unwrap();

        let google = google_sign_in(&mut tx, &s.id, &s.email, "owner", None, &key).await.unwrap();
        let reset = password_reset(&mut tx, &s.id, &s.email, "owner-hash", &key).await.unwrap();

        assert_eq!(google, Claim::Moved);
        assert_eq!(reset, Claim::Moved);
        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("password_hash"), "registrant-hash");
        assert_eq!(row.get::<String, _>("api_key"), s.old_index);
    }

    /// With every four-digit name on a stem taken, a name is still found, so
    /// pre-registering them cannot make a Google claim fail.
    #[tokio::test]
    async fn a_free_username_is_found_when_the_short_ones_are_all_taken() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let stem = format!("taken{}", rng::number(8));
        sqlx::query(
            "INSERT INTO users (id, username, email, password_hash, otp, api_key, account_type) \
             SELECT gen_random_uuid()::text, $1 || n, $1 || n || '@example.com', '', '', \
             'test-index-' || $1 || n, 'personal' FROM generate_series(1000, 9999) AS n",
        )
        .bind(&stem)
        .execute(&mut *tx)
        .await
        .unwrap();

        let name = free_username(&mut tx, &stem).await.unwrap();

        let taken: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM users WHERE username = $1)")
            .bind(&name)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        assert!(!taken, "{name} is already someone's username");
        assert!(name.starts_with(&stem));
    }

    /// A change-password or rotate-api-key request let in on the registrant's
    /// key, whose write runs only after the claim, lands nothing.
    #[tokio::test]
    async fn writes_let_in_on_the_old_key_land_nothing_after_a_claim() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let mut tx = conn.begin().await.unwrap();
        let s = seed(&mut tx, false).await;
        let key = apikey::mint().unwrap();
        google_sign_in(&mut tx, &s.id, &s.email, "owner", None, &key).await.unwrap();

        // change_password reloads the row after the claim: no password, so no
        // current-password check, and it writes against what it read.
        let set = super::super::change_password::set_password(&mut *tx, &s.id, &s.old_index, "", "registrant-new")
            .await
            .unwrap();
        assert!(!set, "a password change on the registrant's key went through");
        let fresh = apikey::mint().unwrap();
        let rotated = super::super::rotate_api_key::rotate_in(
            &mut tx, &s.id, &s.old_index, &fresh.blind_index, &fresh.encrypted,
        )
        .await
        .unwrap();
        assert!(!rotated, "a rotation on the registrant's key handed out a new one");

        let row = user_row(&mut tx, &s.id).await;
        assert_eq!(row.get::<String, _>("password_hash"), "");
        assert_eq!(row.get::<String, _>("api_key"), key.blind_index);
        // The owner's own requests still go through.
        assert!(key_guard::still_current(&mut tx, &s.id, &key.blind_index).await.unwrap());
        assert!(super::super::change_password::set_password(&mut *tx, &s.id, &key.blind_index, "", "owner-new")
            .await
            .unwrap());
    }

    /// A share toggle let in on the registrant's key that reaches its write
    /// while the claim is in progress waits for the claim, then finds the key
    /// gone and publishes nothing. This one needs a second connection, so the
    /// account is committed and deleted again at the end.
    #[tokio::test]
    async fn a_share_toggle_racing_a_claim_publishes_nothing() {
        let Some(mut conn) = connect().await else { return eprintln!("skipped: no SILOCAT_TEST_DATABASE_URL") };
        let url = std::env::var("SILOCAT_TEST_DATABASE_URL").unwrap();
        let mut setup = conn.begin().await.unwrap();
        let s = seed(&mut setup, false).await;
        setup.commit().await.unwrap();

        let mut claim = conn.begin().await.unwrap();
        let key = apikey::mint().unwrap();
        google_sign_in(&mut claim, &s.id, &s.email, "owner", None, &key).await.unwrap();

        // What toggle_share does once the request is past resolve_identity.
        let (id, old_index, folder_id) = (s.id.clone(), s.old_index.clone(), s.folder_id.clone());
        let toggle = tokio::spawn(async move {
            let mut other = PgConnection::connect(&url).await.unwrap();
            let mut tx = other.begin().await.unwrap();
            let allowed = key_guard::still_current(&mut tx, &id, &old_index).await.unwrap();
            if allowed {
                sqlx::query(
                    "UPDATE folders SET share_type = 'public', share_token = 'registrant-new-link', \
                     public_access = true WHERE id::text = $1 AND user_id = $2",
                )
                .bind(&folder_id)
                .bind(&id)
                .execute(&mut *tx)
                .await
                .unwrap();
            }
            tx.commit().await.unwrap();
            allowed
        });
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        claim.commit().await.unwrap();
        let allowed = toggle.await.unwrap();

        let published: bool = sqlx::query_scalar(
            "SELECT share_type <> 'off' OR share_token IS NOT NULL OR public_access FROM folders WHERE id::text = $1",
        )
        .bind(&s.folder_id)
        .fetch_one(&mut conn)
        .await
        .unwrap();
        for sql in [
            "DELETE FROM file_requests WHERE owner_user_id = $1",
            "DELETE FROM files WHERE user_id = $1",
            "DELETE FROM folders WHERE user_id = $1",
            "DELETE FROM users WHERE id = $1",
        ] {
            sqlx::query(sql).bind(&s.id).execute(&mut conn).await.unwrap();
        }
        assert!(!allowed, "the toggle was let through on the registrant's key");
        assert!(!published, "the registrant's toggle republished the folder after the claim");
    }
}
