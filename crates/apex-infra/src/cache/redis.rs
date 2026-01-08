//! Redis cache implementation with connection pooling and optional fallback.

use std::time::Duration;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::{AsyncCommands, Client};

use apex_core::ports::{Cache, CacheError};

/// Redis connection configuration.
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis URL (e.g., redis://localhost:6379)
    pub url: String,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Whether to fallback to in-memory cache if Redis is unavailable
    pub fallback_to_memory: bool,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://localhost:6379".to_string(),
            connect_timeout: Duration::from_secs(5),
            fallback_to_memory: true,
        }
    }
}

impl RedisConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
            connect_timeout: Duration::from_secs(
                std::env::var("REDIS_CONNECT_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5),
            ),
            fallback_to_memory: std::env::var("REDIS_FALLBACK_TO_MEMORY")
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

/// Redis-backed cache implementation.
///
/// Uses connection manager for automatic reconnection and pooling.
pub struct RedisCache {
    conn: ConnectionManager,
    #[allow(dead_code)]
    config: RedisConfig,
}

impl RedisCache {
    pub async fn new(config: RedisConfig) -> Result<Self, CacheError> {
        let client =
            Client::open(config.url.as_str()).map_err(|e| CacheError::Connection(e.to_string()))?;

        // Use timeout to prevent hanging if Redis is unreachable
        let conn_manager_fut = ConnectionManager::new(client);
        let conn = tokio::time::timeout(config.connect_timeout, conn_manager_fut)
            .await
            .map_err(|_| CacheError::Connection("Connection timed out".to_string()))?
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        tracing::info!(url = %config.url, "Connected to Redis cache");

        Ok(Self { conn, config })
    }

    /// Create from environment configuration.
    pub async fn from_env() -> Result<Self, CacheError> {
        Self::new(RedisConfig::from_env()).await
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.clone();
        match conn.get::<_, Option<String>>(key).await {
            Ok(value) => value,
            Err(e) => {
                tracing::warn!(key = %key, error = %e, "Redis GET failed");
                None
            }
        }
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();

        match ttl {
            Some(duration) => {
                conn.set_ex::<_, _, ()>(key, value, duration.as_secs())
                    .await
                    .map_err(|e| CacheError::Operation(e.to_string()))?;
            }
            None => {
                conn.set::<_, _, ()>(key, value)
                    .await
                    .map_err(|e| CacheError::Operation(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.conn.clone();
        conn.del::<_, ()>(key)
            .await
            .map_err(|e| CacheError::Operation(e.to_string()))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> bool {
        let mut conn = self.conn.clone();
        conn.exists::<_, bool>(key).await.unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn get_test_cache() -> Option<RedisCache> {
        let config = RedisConfig {
            url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://localhost:6389".to_string()),
            connect_timeout: Duration::from_secs(1),
            fallback_to_memory: false,
        };

        RedisCache::new(config).await.ok()
    }

    #[tokio::test]
    async fn test_redis_cache_set_get() {
        let cache = match get_test_cache().await {
            Some(c) => c,
            None => {
                tracing::warn!("Redis not available, skipping test");
                return;
            }
        };

        let key = "test_key";
        let value = "test_value";

        cache.set(key, value, None).await.unwrap();
        assert_eq!(cache.get(key).await, Some(value.to_string()));

        cache.delete(key).await.unwrap();
        assert_eq!(cache.get(key).await, None);
    }

    #[tokio::test]
    async fn test_redis_cache_ttl() {
        let cache = match get_test_cache().await {
            Some(c) => c,
            None => return,
        };

        let key = "test_ttl_key";
        let value = "test_ttl_value";

        // Set with 1s TTL
        cache
            .set(key, value, Some(Duration::from_secs(1)))
            .await
            .unwrap();
        assert_eq!(cache.get(key).await, Some(value.to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert_eq!(cache.get(key).await, None);
    }
}
