//! Application state - shared across all handlers.

use std::sync::Arc;

use apex_core::ports::{Cache, UserRepository};
use apex_infra::cache::InMemoryCache;
use apex_infra::database::{DatabaseConfig, DatabaseConnections};

#[cfg(feature = "postgres")]
use apex_infra::database::PostgresUserRepository;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<dyn Cache>,
    pub user_repo: Arc<dyn UserRepository>,
    pub db: Option<Arc<DatabaseConnections>>,
}

/// In-memory user repository for when database is not configured.
pub struct InMemoryUserRepository;

#[async_trait::async_trait]
impl UserRepository for InMemoryUserRepository {
    async fn find_by_id(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<apex_core::domain::User>, apex_core::error::RepoError> {
        tracing::warn!("Database not configured - using in-memory fallback");
        Ok(None)
    }

    async fn find_by_email(
        &self,
        _email: &str,
    ) -> Result<Option<apex_core::domain::User>, apex_core::error::RepoError> {
        Ok(None)
    }

    async fn save(
        &self,
        user: apex_core::domain::User,
    ) -> Result<apex_core::domain::User, apex_core::error::RepoError> {
        Ok(user)
    }

    async fn delete(&self, _id: uuid::Uuid) -> Result<(), apex_core::error::RepoError> {
        Ok(())
    }
}

impl AppState {
    /// Build the application state with appropriate implementations.
    pub async fn new(db_config: Option<&DatabaseConfig>) -> Self {
        // Initialize cache (in-memory for now, Redis later)
        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new());

        // Initialize database connections if configured
        #[cfg(feature = "postgres")]
        let (db, user_repo): (Option<Arc<DatabaseConnections>>, Arc<dyn UserRepository>) = {
            if let Some(config) = db_config {
                match DatabaseConnections::init(config).await {
                    Ok(connections) => {
                        let conn = Arc::new(connections);
                        let repo = Arc::new(PostgresUserRepository::new(conn.main.clone()));
                        (Some(conn), repo)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to connect to database: {}. Using in-memory fallback.",
                            e
                        );
                        (None, Arc::new(InMemoryUserRepository))
                    }
                }
            } else {
                tracing::warn!("DATABASE_URL not set. Running without database (in-memory mode).");
                (None, Arc::new(InMemoryUserRepository))
            }
        };

        #[cfg(not(feature = "postgres"))]
        let (db, user_repo): (Option<Arc<DatabaseConnections>>, Arc<dyn UserRepository>) = {
            tracing::info!("Running without postgres feature - using in-memory repository");
            (None, Arc::new(InMemoryUserRepository))
        };

        tracing::info!("Application state initialized");

        Self {
            cache,
            user_repo,
            db,
        }
    }
}
