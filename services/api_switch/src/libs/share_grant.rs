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

/// How long a grant lasts for a folder of a few small files.
pub const GRANT_BASE_SECS: i64 = 30 * 60;

/// The most any grant lasts, however large the folder.
pub const GRANT_MAX_SECS: i64 = 6 * 60 * 60;

/// The slowest link a download is sized for, 2 Mbit/s.
const SLOW_LINK_BYTES_PER_SEC: i64 = 256 * 1024;

/// How long a grant for a folder of `total_bytes` lasts.
///
/// A grant is a bearer value: whoever spent the link can hand it on, with the
/// token, until it expires. A flat six hours made that window 36 times the
/// life of a presigned URL for every folder, when most folders need minutes.
/// The page fetches each file just before downloading it, so the grant has to
/// outlast the fetch of the last file, which is the folder's size at a slow
/// link's speed, and nothing past that.
pub fn ttl_for(total_bytes: i64) -> i64 {
    (GRANT_BASE_SECS + total_bytes.max(0) / SLOW_LINK_BYTES_PER_SEC).min(GRANT_MAX_SECS)
}

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

fn issue_with(key: &[u8; 32], folder_id: &str, token: &str, now: i64, ttl: i64) -> Option<String> {
    let expires = now + ttl.clamp(1, GRANT_MAX_SECS);
    let tag = mac(key, folder_id, token, expires)?.finalize().into_bytes();
    Some(format!("{}.{}", expires, hex::encode(tag)))
}

fn verify_with(key: &[u8; 32], grant: &str, folder_id: &str, token: &str, now: i64) -> bool {
    let Some((expires, tag)) = grant.split_once('.') else { return false };
    let Ok(expires) = expires.parse::<i64>() else { return false };
    if expires <= now || expires > now + GRANT_MAX_SECS {
        return false;
    }
    let Ok(tag) = hex::decode(tag) else { return false };
    match mac(key, folder_id, token, expires) {
        Some(m) => m.verify_slice(&tag).is_ok(),
        None => false,
    }
}

/// A grant for one spend of `token` on `folder_id`, whose files come to
/// `total_bytes`. `None` only when the service has no key configured.
pub fn issue(folder_id: &str, token: &str, total_bytes: i64) -> Option<String> {
    issue_with(key()?, folder_id, token, chrono::Utc::now().timestamp(), ttl_for(total_bytes))
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
        let g = issue_with(&K, "fold-1", "tok-1", NOW, GRANT_MAX_SECS).unwrap();
        assert!(verify_with(&K, &g, "fold-1", "tok-1", NOW));
        assert!(verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_MAX_SECS - 1));
    }

    #[test]
    fn grant_does_not_carry_over_to_another_folder_or_token() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW, GRANT_MAX_SECS).unwrap();
        assert!(!verify_with(&K, &g, "fold-2", "tok-1", NOW));
        assert!(!verify_with(&K, &g, "fold-1", "tok-2", NOW), "a regenerated link revokes old grants");
    }

    #[test]
    fn grant_expires() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW, GRANT_MAX_SECS).unwrap();
        assert!(!verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_MAX_SECS));
    }

    #[test]
    fn extending_the_expiry_breaks_the_mac() {
        let g = issue_with(&K, "fold-1", "tok-1", NOW, GRANT_MAX_SECS).unwrap();
        let (_, tag) = g.split_once('.').unwrap();
        let forged = format!("{}.{}", NOW + GRANT_MAX_SECS + 100, tag);
        assert!(!verify_with(&K, &forged, "fold-1", "tok-1", NOW + GRANT_MAX_SECS + 50));
        let in_window = format!("{}.{}", NOW + 60, tag);
        assert!(!verify_with(&K, &in_window, "fold-1", "tok-1", NOW));
    }

    #[test]
    fn far_future_expiry_is_refused_even_before_the_mac() {
        let forged = format!("{}.{}", NOW + 10 * GRANT_MAX_SECS, "00".repeat(32));
        assert!(!verify_with(&K, &forged, "fold-1", "tok-1", NOW));
    }

    #[test]
    fn junk_and_other_keys_are_refused() {
        for junk in ["", "x", ".", "123.", ".abcd", "abc.def", "9999999999.zz"] {
            assert!(!verify_with(&K, junk, "fold-1", "tok-1", NOW), "{junk:?}");
        }
        let g = issue_with(&[8u8; 32], "fold-1", "tok-1", NOW, GRANT_MAX_SECS).unwrap();
        assert!(!verify_with(&K, &g, "fold-1", "tok-1", NOW));
    }

    #[test]
    fn a_small_folder_gets_the_base_window() {
        assert_eq!(ttl_for(0), GRANT_BASE_SECS);
        assert_eq!(ttl_for(10 << 20), GRANT_BASE_SECS + 40);
        assert_eq!(ttl_for(-5), GRANT_BASE_SECS);
        let g = issue_with(&K, "fold-1", "tok-1", NOW, ttl_for(1 << 20)).unwrap();
        assert!(verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_BASE_SECS - 1));
        assert!(!verify_with(&K, &g, "fold-1", "tok-1", NOW + GRANT_BASE_SECS + 10));
    }

    #[test]
    fn a_large_folder_gets_time_for_a_slow_link_up_to_the_cap() {
        let three_gb = 3 * (1i64 << 30);
        assert_eq!(ttl_for(three_gb), GRANT_BASE_SECS + three_gb / SLOW_LINK_BYTES_PER_SEC);
        assert_eq!(ttl_for(1i64 << 40), GRANT_MAX_SECS);
        assert_eq!(ttl_for(i64::MAX), GRANT_MAX_SECS);
    }
}
