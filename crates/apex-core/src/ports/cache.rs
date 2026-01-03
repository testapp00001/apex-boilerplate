use async_trait::async_trait;
use std::time::Duration;

/// Cache trait - abstraction over caching backends (Redis, in-memory).
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get a value from the cache.
    async fn get(&self, key: &str) -> Option<String>;

    /// Set a value in the cache with optional TTL.
    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), CacheError>;

    /// Delete a key from the cache.
    async fn delete(&self, key: &str) -> Result<(), CacheError>;

    /// Check if a key exists.
    async fn exists(&self, key: &str) -> bool;
}

/// Cache operation errors.
#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Operation failed: {0}")]
    Operation(String),
}
