//! Proof that a one-time folder link was spent.
//!
//! A folder download is one authorize call followed by one fetch-chunks call per
//! file. Authorize is where a one-time link's counter goes up, so fetch-chunks
//! cannot decide from the counter alone: before the spend the counter says "go"
//! to anyone who skips authorize, and after it the counter says "spent" to the
//! very recipient whose zip is still being built.
//!
//! So authorize hands back a grant: an expiry plus an HMAC over the folder id,
//! the share token and that expiry. fetch-chunks on a one-time folder serves a
//! file only to a caller holding a valid grant for that folder and token. The
//! token is in the MAC so regenerating the link also revokes grants already
//! handed out.
//!
//! The key is derived from `API_KEY_ENC_KEY`, which the service already refuses
//! to start without, with its own label so it shares nothing with the API key
//! index or cipher.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// How long a grant lasts. Long enough for a zip of a large folder to be built
/// one file at a time, short enough that a grant is a download session rather
/// than a second, permanent link.
pub const GRANT_TTL_SECS: i64 = 6 * 60 * 60;

static KEY: OnceLock<Option<[u8; 32]>> = OnceLock::new();

fn key() -> Option<&'static [u8; 32]> {
    KEY.get_or_init(|| {
        let master = std::env::var("API_KEY_ENC_KEY").ok()?;
        let master = master.trim();
        if master.is_empty() {
            return None;
        }
        let mut h = Sha256::new();
        h.update(master.as_bytes());
        h.update(b"silocat:share:folder-grant");
        Some(h.finalize().into())
    })
    .as_ref()
}

fn mac(key: &[u8; 32], folder_id: &str, token: &str, expires: i64) -> Option<Hmac<Sha256>> {
    let mut m = <Hmac<Sha256> as Mac>::new_from_slice(key).ok()?;
    m.update(b"v1\0");
    m.update(folder_id.as_bytes());
    m.update(b"\0");
    m.update(token.as_bytes());
    m.update(b"\0");
    m.update(expires.to_string().as_bytes());
    Some(m)
}

fn issue_with(key: &[u8; 32], folder_id: &str, token: &str, now: i64) -> Option<String> {
    let expires = now + GRANT_TTL_SECS;
    let tag = mac(key, folder_id, token, expires)?.finalize().into_bytes();
    Some(format!("{}.{}", expires, hex::encode(tag)))
}

fn verify_with(key: &[u8; 32], grant: &str, folder_id: &str, token: &str, now: i64) -> bool {
    let Some((expires, tag)) = grant.split_once('.') else { return false };
    let Ok(expires) = expires.parse::<i64>() else { return false };
    if expires <= now || expires > now + GRANT_TTL_SECS {
        return false;
    }
    let Ok(tag) = hex::decode(tag) else { return false };
    match mac(key, folder_id, token, expires) {
        Some(m) => m.verify_slice(&tag).is_ok(),
        None => false,
    }
}

/// A grant for one spend of `token` on `folder_id`. `None` only when the
/// service has no key configured.
pub fn issue(folder_id: &str, token: &str) -> Option<String> {
    issue_with(key()?, folder_id, token, chrono::Utc::now().timestamp())
}

/// Whether `grant` was issued for this folder and token and has not expired.
/// Without a configured key nothing verifies.
pub fn verify(grant: &str, folder_id: &str, token: &str) -> bool {
    match key() {
        Some(k) => verify_with(k, grant, folder_id, token, chrono::Utc::now().timestamp()),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: [u8; 32] = [7u8; 32];
    const NOW: i64 = 1_800_000_000;

    #[test]
    fn issued_grant_verifies_for_its_folder_and_token() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW).unwrap();
        assert!(verify_with(&K, &g, "fold-1", "tok-1", NOW));
        assert!(verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_TTL_SECS - 1));
    }

    #[test]
    fn grant_does_not_carry_over_to_another_folder_or_token() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW).unwrap();
        assert!(!verify_with(&K, &g, "fold-2", "tok-1", NOW));
        assert!(!verify_with(&K, &g, "fold-1", "tok-2", NOW), "a regenerated link revokes old grants");
    }

    #[test]
    fn grant_expires() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW).unwrap();
        assert!(!verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_TTL_SECS));
    }

    #[test]
    fn extending_the_expiry_breaks_the_mac() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW).unwrap();
        let (_, tag) = g.split_once('.').unwrap();
        let forged = format!("{}.{}", NOW + GRANT_TTL_SECS + 100, tag);
        assert!(!verify_with(&K, &forged, "fold-1", "tok-1", NOW + GRANT_TTL_SECS + 50));
        let in_window = format!("{}.{}", NOW + 60, tag);
        assert!(!verify_with(&K, &in_window, "fold-1", "tok-1", NOW));
    }

    #[test]
    fn far_future_expiry_is_refused_even_before_the_mac() {
        let forged = format!("{}.{}", NOW + 10 * GRANT_TTL_SECS, "00".repeat(32));
        assert!(!verify_with(&K, &forged, "fold-1", "tok-1", NOW));
    }

    #[test]
    fn junk_and_other_keys_are_refused() {
        for junk in ["", "x", ".", "123.", ".abcd", "abc.def", "9999999999.zz"] {
            assert!(!verify_with(&K, junk, "fold-1", "tok-1", NOW), "{junk:?}");
        }
        let g = issue_with(&[8u8; 32], "fold-1", "tok-1", NOW).unwrap();
        assert!(!verify_with(&K, &g, "fold-1", "tok-1", NOW));
    }
}
