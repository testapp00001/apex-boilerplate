//! Rate limiting port.

use async_trait::async_trait;
use std::time::Duration;

/// Rate limiter trait - abstraction over rate limiting backends.
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Check if a request is allowed and update the counter.
    /// Returns Ok(true) if allowed, Ok(false) if rate limited.
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError>;
}

/// Result of a rate limit check.
#[derive(Debug, Clone)]
pub struct RateLimitResult {
    pub allowed: bool,
    pub remaining: u32,
    pub reset_after: Duration,
}

/// Rate limit errors.
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Backend error: {0}")]
    Backend(String),
}
