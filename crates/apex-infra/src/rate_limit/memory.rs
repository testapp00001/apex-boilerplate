//! In-memory rate limiter using governor crate.

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use governor::clock::DefaultClock;
use governor::state::{InMemoryState, NotKeyed};
use governor::{Quota, RateLimiter as GovernorRateLimiter};

use apex_core::ports::{RateLimitError, RateLimitResult, RateLimiter};

type DirectRateLimiter = GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock>;

/// In-memory rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window.
    pub max_requests: u32,
    /// Window duration.
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
        }
    }
}

/// In-memory rate limiter using the GCRA algorithm.
///
/// This is the fallback when Redis is not available.
/// Note: Limits are per-process, not distributed across instances.
pub struct InMemoryRateLimiter {
    limiter: Arc<DirectRateLimiter>,
    config: RateLimitConfig,
}

impl InMemoryRateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = Quota::with_period(config.window / config.max_requests)
            .expect("Valid quota")
            .allow_burst(NonZeroU32::new(config.max_requests).expect("Non-zero"));

        let limiter = Arc::new(DirectRateLimiter::direct(quota));

        Self { limiter, config }
    }

    pub fn from_env() -> Self {
        let config = RateLimitConfig {
            max_requests: std::env::var("RATE_LIMIT_MAX_REQUESTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            window: Duration::from_secs(
                std::env::var("RATE_LIMIT_WINDOW_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(60),
            ),
        };
        Self::new(config)
    }
}

#[async_trait]
impl RateLimiter for InMemoryRateLimiter {
    async fn check(&self, _key: &str) -> Result<RateLimitResult, RateLimitError> {
        // Note: InMemoryRateLimiter is global, not per-key
        // For per-key limiting, use the keyed variant with DashMap
        match self.limiter.check() {
            Ok(_) => Ok(RateLimitResult {
                allowed: true,
                remaining: self.config.max_requests, // Approximate
                reset_after: self.config.window,
            }),
            Err(not_until) => Ok(RateLimitResult {
                allowed: false,
                remaining: 0,
                reset_after: not_until.wait_time_from(governor::clock::Clock::now(
                    &governor::clock::DefaultClock::default(),
                )),
            }),
        }
    }
}
