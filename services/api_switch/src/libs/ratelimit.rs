//! Minimal in-process per-key rate limiter (fixed window).
//!
//! A defense-in-depth throttle for auth/abuse endpoints on top of Cloudflare's
//! edge rate limiting. Keys are `"<bucket>:<client-ip>"`, where the IP is the
//! Cloudflare-set `cf-connecting-ip` (clients can't spoof it behind CF), so one
//! IP can't brute-force login / password-reset / spam OTPs. Not distributed: //! per-process: which is sufficient as a second layer.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(HashMap::new())) }
    }

    /// Returns true if the call is allowed, false if `key` has exceeded `max`
    /// hits within `window`. Fixed window: the count resets once the window
    /// since first hit elapses.
    pub fn check(&self, key: &str, max: u32, window: Duration) -> bool {
        let now = Instant::now();
        let mut map = match self.inner.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(), // poisoned: recover rather than panic in a handler
        };

        // Opportunistic prune so the map can't grow without bound.
        if map.len() > 10_000 {
            map.retain(|_, (_, start)| now.duration_since(*start) < window);
        }

        let entry = map.entry(key.to_string()).or_insert((0, now));
        if now.duration_since(entry.1) > window {
            *entry = (1, now);
            true
        } else {
            entry.0 = entry.0.saturating_add(1);
            entry.0 <= max
        }
    }

    /// True while `key` has fewer than `max` hits in its current window. Counts
    /// nothing; pair it with `hit` to meter only some outcomes, such as failures.
    pub fn allows(&self, key: &str, max: u32, window: Duration) -> bool {
        let map = match self.inner.lock() {
            Ok(m) => m,
            Err(p) => p.into_inner(),
        };
        match map.get(key) {
            Some((count, start)) if Instant::now().duration_since(*start) <= window => *count < max,
            _ => true,
        }
    }

    /// Count one hit against `key`, starting a new window if the last one ran out.
    pub fn hit(&self, key: &str, window: Duration) {
        let _ = self.check(key, u32::MAX, window);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_counts_nothing_and_hit_counts_one() {
        let limiter = RateLimiter::new();
        let window = Duration::from_secs(300);
        for _ in 0..50 {
            assert!(limiter.allows("k", 3, window));
        }
        limiter.hit("k", window);
        limiter.hit("k", window);
        assert!(limiter.allows("k", 3, window));
        limiter.hit("k", window);
        assert!(!limiter.allows("k", 3, window));
        assert!(limiter.allows("other", 3, window));
    }
}
