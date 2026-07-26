//! API keys at rest.
//!
//! An API key is a bearer credential, so a database dump should not hand one
//! over. But users need to read their key back in Settings, which rules out a
//! plain hash. So each key is stored twice:
//!
//!   - **blind index** (`api_key` column): HMAC-SHA256 of the key. Deterministic,
//!     so it can be indexed and looked up in one query, and irreversible on its
//!     own. This is what every lookup and every ownership comparison uses.
//!   - **ciphertext** (`api_key_enc` column): XChaCha20-Poly1305, random nonce
//!     per write. Only ever decrypted to show the user their own key.
//!
//! Both subkeys are derived from `API_KEY_ENC_KEY` so the index and the cipher
//! never share key material. A dump without that secret yields neither the keys
//! nor a way to test a guess against them.
//!
//! Anonymous (shadow) keys are generated in the browser and never displayed by
//! the server, so those rows store the blind index only.

use chacha20poly1305::{
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
    XChaCha20Poly1305, XNonce,
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

const NONCE_LEN: usize = 24;

struct Keys {
    bi: [u8; 32],
    enc: [u8; 32],
}

static KEYS: OnceLock<Option<Keys>> = OnceLock::new();

fn keys() -> Option<&'static Keys> {
    KEYS.get_or_init(|| {
        let master = std::env::var("API_KEY_ENC_KEY").ok()?;
        let master = master.trim();
        if master.is_empty() {
            return None;
        }
        // Domain-separated subkeys: the index key must not also be the cipher key.
        let derive = |label: &[u8]| -> [u8; 32] {
            let mut h = Sha256::new();
            h.update(master.as_bytes());
            h.update(label);
            h.finalize().into()
        };
        Some(Keys { bi: derive(b"silocat:apikey:blind-index"), enc: derive(b"silocat:apikey:encrypt") })
    })
    .as_ref()
}

/// True when `API_KEY_ENC_KEY` is configured. Checked once at startup so the
/// process refuses to serve rather than silently falling back to plaintext.
pub fn is_configured() -> bool {
    keys().is_some()
}

/// Deterministic, irreversible lookup value for a raw key.
///
/// Returns `None` only when the service is misconfigured, which callers must
/// treat as "no match" rather than "any match".
pub fn blind_index(raw: &str) -> Option<String> {
    let k = keys()?;
    // qualified: chacha's KeyInit also exposes new_from_slice
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(&k.bi).ok()?;
    mac.update(raw.trim().as_bytes());
    Some(hex::encode(mac.finalize().into_bytes()))
}

/// Encrypt a raw key for storage. Output is base64(nonce || ciphertext).
pub fn encrypt(raw: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let k = keys()?;
    let cipher = XChaCha20Poly1305::new((&k.enc).into());

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = XNonce::from_slice(&nonce_bytes);

    let ct = cipher.encrypt(nonce, raw.as_bytes()).ok()?;
    let mut out = Vec::with_capacity(NONCE_LEN + ct.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ct);
    Some(STANDARD.encode(out))
}

/// Decrypt a stored key for display back to its owner.
pub fn decrypt(stored: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let k = keys()?;
    let raw = STANDARD.decode(stored.trim()).ok()?;
    if raw.len() <= NONCE_LEN {
        return None;
    }
    let (nonce_bytes, ct) = raw.split_at(NONCE_LEN);
    let cipher = XChaCha20Poly1305::new((&k.enc).into());
    let pt = cipher.decrypt(XNonce::from_slice(nonce_bytes), ct).ok()?;
    String::from_utf8(pt).ok()
}

/// A fresh raw key, plus the two values that get stored for it.
pub struct NewKey {
    pub raw: String,
    pub blind_index: String,
    pub encrypted: String,
}

/// Mint a new API key. `None` when the service is misconfigured.
pub fn mint() -> Option<NewKey> {
    let raw = crate::libs::rng::uuid();
    Some(NewKey {
        blind_index: blind_index(&raw)?,
        encrypted: encrypt(&raw)?,
        raw,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_key<T>(f: impl FnOnce() -> T) -> T {
        // OnceLock means the first test to touch it fixes the key for the
        // process, which is fine: every test here only needs *a* valid key.
        unsafe { std::env::set_var("API_KEY_ENC_KEY", "test-master-key-for-unit-tests") };
        f()
    }

    #[test]
    fn blind_index_is_deterministic_and_not_the_key() {
        with_key(|| {
            let a = blind_index("abc-123").unwrap();
            let b = blind_index("abc-123").unwrap();
            assert_eq!(a, b, "same key must map to the same index");
            assert_ne!(a, "abc-123");
            assert_ne!(blind_index("abc-124").unwrap(), a);
            assert_eq!(a.len(), 64, "hex sha256");
        });
    }

    #[test]
    fn blind_index_trims_like_lookups_do() {
        with_key(|| assert_eq!(blind_index("  k  ").unwrap(), blind_index("k").unwrap()));
    }

    #[test]
    fn encrypt_roundtrips_and_is_nondeterministic() {
        with_key(|| {
            let a = encrypt("secret-key").unwrap();
            let b = encrypt("secret-key").unwrap();
            assert_ne!(a, b, "random nonce per write");
            assert_eq!(decrypt(&a).unwrap(), "secret-key");
            assert_eq!(decrypt(&b).unwrap(), "secret-key");
        });
    }

    #[test]
    fn decrypt_rejects_garbage_and_truncation() {
        with_key(|| {
            assert!(decrypt("not-base64!!").is_none());
            assert!(decrypt("").is_none());
            let good = encrypt("x").unwrap();
            assert!(decrypt(&good[..good.len() / 2]).is_none());
        });
    }

    #[test]
    fn mint_produces_consistent_triple() {
        with_key(|| {
            let k = mint().unwrap();
            assert_eq!(blind_index(&k.raw).unwrap(), k.blind_index);
            assert_eq!(decrypt(&k.encrypted).unwrap(), k.raw);
        });
    }
}
