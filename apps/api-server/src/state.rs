//! Application state - shared across all handlers.

use std::sync::Arc;

use apex_core::ports::Cache;
use apex_infra::cache::InMemoryCache;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<dyn Cache>,
    // Database connections will be added when postgres feature is enabled
    // pub db: Arc<DatabaseConnections>,
}

impl AppState {
    /// Build the application state with appropriate implementations.
    pub async fn new() -> Self {
        // For now, use in-memory cache
        // When database is configured, we'll add the connection initialization here
        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new());

        tracing::info!("Application state initialized");

        Self { cache }
    }
}
