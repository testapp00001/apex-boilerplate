//! In-memory cache implementation - used as fallback when Redis is unavailable.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::sync::RwLock;

use apex_core::ports::{Cache, CacheError};

struct CacheEntry {
    value: String,
    expires_at: Option<Instant>,
}

/// In-memory cache using a simple HashMap with async RwLock.
///
/// This is the fallback implementation when Redis is not available.
/// Note: Data is lost on process restart.
pub struct InMemoryCache {
    store: RwLock<HashMap<String, CacheEntry>>,
}

impl InMemoryCache {
    pub fn new() -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
        }
    }

    fn is_expired(entry: &CacheEntry) -> bool {
        entry
            .expires_at
            .map(|exp| Instant::now() > exp)
            .unwrap_or(false)
    }
}

impl Default for InMemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for InMemoryCache {
    async fn get(&self, key: &str) -> Option<String> {
        // Use async read lock - doesn't block the executor
        let store = self.store.read().await;
        let entry = store.get(key)?;

        if Self::is_expired(entry) {
            drop(store);
            // Clean up expired entry with write lock
            let mut store = self.store.write().await;
            store.remove(key);
            return None;
        }

        Some(entry.value.clone())
    }

    async fn set(&self, key: &str, value: &str, ttl: Option<Duration>) -> Result<(), CacheError> {
        // Use async write lock - doesn't block the executor
        let mut store = self.store.write().await;

        let expires_at = ttl.map(|d| Instant::now() + d);

        store.insert(
            key.to_string(),
            CacheEntry {
                value: value.to_string(),
                expires_at,
            },
        );

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut store = self.store.write().await;
        store.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> bool {
        self.get(key).await.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let cache = InMemoryCache::new();
        cache.set("key1", "value1", None).await.unwrap();
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let cache = InMemoryCache::new();
        cache.set("key1", "value1", None).await.unwrap();
        cache.delete("key1").await.unwrap();
        assert_eq!(cache.get("key1").await, None);
    }
}
