//! Redis rate limiter implementation using sliding window counter.

use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{Client, Script};

use apex_core::ports::{RateLimitError, RateLimitResult, RateLimiter};

use crate::cache::RedisConfig;

/// Redis rate limiter configuration.
#[derive(Debug, Clone)]
pub struct RedisRateLimitConfig {
    /// Redis connection config
    pub redis: RedisConfig,
    /// Maximum requests per window
    pub max_requests: u32,
    /// Window duration
    pub window: Duration,
    /// Key prefix for rate limit keys
    pub key_prefix: String,
}

impl Default for RedisRateLimitConfig {
    fn default() -> Self {
        Self {
            redis: RedisConfig::default(),
            max_requests: 100,
            window: Duration::from_secs(60),
            key_prefix: "ratelimit".to_string(),
        }
    }
}

impl RedisRateLimitConfig {
    pub fn from_env() -> Self {
        Self {
            redis: RedisConfig::from_env(),
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
            key_prefix: std::env::var("RATE_LIMIT_KEY_PREFIX")
                .unwrap_or_else(|_| "ratelimit".to_string()),
        }
    }
}

/// Redis-backed rate limiter using sliding window counter.
pub struct RedisRateLimiter {
    conn: ConnectionManager,
    config: RedisRateLimitConfig,
    /// Lua script for atomic increment with expiry
    script: Script,
}

impl RedisRateLimiter {
    pub async fn new(config: RedisRateLimitConfig) -> Result<Self, RateLimitError> {
        let client = Client::open(config.redis.url.as_str())
            .map_err(|e| RateLimitError::Backend(e.to_string()))?;

        // Use timeout to prevent hanging if Redis is unreachable
        let conn_manager_fut = ConnectionManager::new(client);
        let conn = tokio::time::timeout(config.redis.connect_timeout, conn_manager_fut)
            .await
            .map_err(|_| RateLimitError::Backend("Connection timed out".to_string()))?
            .map_err(|e| RateLimitError::Backend(e.to_string()))?;

        // Lua script for atomic increment with TTL
        // Returns: [current_count, ttl_remaining]
        let script = Script::new(
            r#"
            local key = KEYS[1]
            local max_requests = tonumber(ARGV[1])
            local window_secs = tonumber(ARGV[2])
            
            local current = redis.call('INCR', key)
            if current == 1 then
                redis.call('EXPIRE', key, window_secs)
            end
            
            local ttl = redis.call('TTL', key)
            return {current, ttl}
            "#,
        );

        tracing::info!(url = %config.redis.url, "Connected to Redis rate limiter");

        Ok(Self {
            conn,
            config,
            script,
        })
    }

    /// Create from environment configuration.
    pub async fn from_env() -> Result<Self, RateLimitError> {
        Self::new(RedisRateLimitConfig::from_env()).await
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }
}

#[async_trait]
impl RateLimiter for RedisRateLimiter {
    async fn check(&self, key: &str) -> Result<RateLimitResult, RateLimitError> {
        let redis_key = self.make_key(key);
        let mut conn = self.conn.clone();

        let result: Vec<i64> = self
            .script
            .key(&redis_key)
            .arg(self.config.max_requests)
            .arg(self.config.window.as_secs())
            .invoke_async(&mut conn)
            .await
            .map_err(|e| RateLimitError::Backend(e.to_string()))?;

        let current_count = result.first().copied().unwrap_or(1) as u32;
        let ttl_secs = result.get(1).copied().unwrap_or(60).max(1) as u64;

        let allowed = current_count <= self.config.max_requests;
        let remaining = if allowed {
            self.config.max_requests.saturating_sub(current_count)
        } else {
            0
        };

        Ok(RateLimitResult {
            allowed,
            remaining,
            reset_after: Duration::from_secs(ttl_secs),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn get_test_ratelimiter() -> Option<RedisRateLimiter> {
        let config = RedisRateLimitConfig {
            redis: RedisConfig {
                url: std::env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6389".to_string()),
                connect_timeout: Duration::from_secs(1),
                fallback_to_memory: false,
            },
            max_requests: 2,
            window: Duration::from_secs(1),
            key_prefix: "test_ratelimit".to_string(),
        };

        RedisRateLimiter::new(config).await.ok()
    }

    #[tokio::test]
    async fn test_redis_ratelimiter() {
        let limiter = match get_test_ratelimiter().await {
            Some(l) => l,
            None => return,
        };

        let key = "test_user_1";

        // First request - allowed
        let res = limiter.check(key).await.unwrap();
        assert!(res.allowed);
        assert_eq!(res.remaining, 1);

        // Second request - allowed
        let res = limiter.check(key).await.unwrap();
        assert!(res.allowed);
        assert_eq!(res.remaining, 0);

        // Third request - rejected
        let res = limiter.check(key).await.unwrap();
        assert!(!res.allowed);

        // Wait for reset
        tokio::time::sleep(Duration::from_millis(1500)).await;

        // Fourth request - allowed again
        let res = limiter.check(key).await.unwrap();
        assert!(res.allowed);
    }
}
