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
    fn test_rate_limiter() {
        let limiter = RateLimiter::new();

        // First request should pass
        assert!(limiter.check(123));

        // Immediate second request should fail (if interval > 0)
        if RATE_LIMIT_SECONDS > 0 {
            assert!(!limiter.check(123));
        }

        // Different user should pass
        assert!(limiter.check(456));
    }
}
