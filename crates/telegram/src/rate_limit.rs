//! Simple rate limiting for user commands
//!
//! Prevents abuse by limiting command frequency per user.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use crate::constants::RATE_LIMIT_SECONDS;

/// Thread-safe rate limiter using user IDs
pub struct RateLimiter {
    /// Map of user ID to last command timestamp
    last_command: RwLock<HashMap<u64, Instant>>,
    /// Minimum interval between commands
    interval: Duration,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    /// Create a new rate limiter with default interval
    pub fn new() -> Self {
        Self {
            last_command: RwLock::new(HashMap::new()),
            interval: Duration::from_secs(RATE_LIMIT_SECONDS),
        }
    }

    /// Check if a user is rate limited
    ///
    /// Returns `true` if the user can proceed, `false` if rate limited.
    /// Updates the last command time if not rate limited.
    pub fn check(&self, user_id: u64) -> bool {
        let now = Instant::now();

        // First try to read
        {
            let last = self.last_command.read().unwrap_or_else(|e| e.into_inner());
            if let Some(&last_time) = last.get(&user_id) {
                if now.duration_since(last_time) < self.interval {
                    return false;
                }
            }
        }

        // Update timestamp
        {
            let mut last = self.last_command.write().unwrap_or_else(|e| e.into_inner());
            last.insert(user_id, now);
        }

        true
    }

    /// Clean up old entries (call periodically)
    pub fn cleanup(&self) {
        let now = Instant::now();
        let cleanup_threshold = Duration::from_secs(60);

        let mut last = self.last_command.write().unwrap_or_else(|e| e.into_inner());
        last.retain(|_, &mut instant| now.duration_since(instant) < cleanup_threshold);
    }
}

/// Global rate limiter instance
static RATE_LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();

/// Get the global rate limiter
pub fn rate_limiter() -> &'static RateLimiter {
    RATE_LIMITER.get_or_init(RateLimiter::new)
}

/// Check if a user is rate limited
///
/// Returns `true` if the user can proceed, `false` if rate limited.
pub fn check_rate_limit(user_id: u64) -> bool {
    rate_limiter().check(user_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_first_request() {
        let limiter = RateLimiter::new();
        // First request should always pass
        assert!(limiter.check(123));
    }

    #[test]
    fn test_rate_limiter_immediate_second_request() {
        let limiter = RateLimiter::new();
        limiter.check(123);

        // Immediate second request should fail (if interval > 0)
        if RATE_LIMIT_SECONDS > 0 {
            assert!(!limiter.check(123));
        }
    }

    #[test]
    fn test_rate_limiter_different_users() {
        let limiter = RateLimiter::new();

        // Both users should pass on first request
        assert!(limiter.check(123));
        assert!(limiter.check(456));
        assert!(limiter.check(789));

        // All should be rate limited on immediate second request
        if RATE_LIMIT_SECONDS > 0 {
            assert!(!limiter.check(123));
            assert!(!limiter.check(456));
            assert!(!limiter.check(789));
        }
    }

    #[test]
    fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new();

        // Add some users
        limiter.check(123);
        limiter.check(456);
        limiter.check(789);

        // Cleanup should not panic
        limiter.cleanup();

        // Fresh entries should still be present (cleanup threshold is 60 seconds)
        // They will be rate limited
        if RATE_LIMIT_SECONDS > 0 {
            assert!(!limiter.check(123));
        }
    }

    #[test]
    fn test_rate_limiter_default() {
        let limiter: RateLimiter = Default::default();
        assert!(limiter.check(999));
    }

    #[test]
    fn test_global_rate_limiter() {
        // Test that global rate limiter is accessible
        let _limiter = rate_limiter();
    }

    #[test]
    fn test_check_rate_limit_function() {
        // This may already be rate limited from previous tests
        // Just verify it doesn't panic
        let _result = check_rate_limit(12345);
    }

    #[test]
    fn test_rate_limiter_thread_safety() {
        use std::thread;

        let limiter = std::sync::Arc::new(RateLimiter::new());
        let mut handles = vec![];

        // Spawn multiple threads accessing the rate limiter
        for i in 0..10 {
            let limiter = limiter.clone();
            handles.push(thread::spawn(move || {
                for j in 0..100 {
                    let user_id = (i * 100 + j) as u64;
                    let _ = limiter.check(user_id);
                }
            }));
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Cleanup should work after concurrent access
        limiter.cleanup();
    }
}
