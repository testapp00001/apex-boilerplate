//! Application state - shared across all handlers.

use std::sync::Arc;

use apex_core::ports::{Cache, PostRepository, UserRepository};
use apex_infra::cache::InMemoryCache;
use apex_infra::database::{DatabaseConfig, DatabaseConnections};

#[cfg(feature = "postgres")]
use apex_infra::database::{PostgresPostRepository, PostgresUserRepository};

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub cache: Arc<dyn Cache>,
    pub users: Arc<dyn UserRepository>,
    pub posts: Arc<dyn PostRepository>,
    pub db: Option<Arc<DatabaseConnections>>,
}

/// In-memory user repository (Stub for when DB is missing)
pub struct StubUserRepository;
#[async_trait::async_trait]
impl apex_core::ports::BaseRepository<apex_core::domain::User, uuid::Uuid> for StubUserRepository {
    async fn find_by_id(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<apex_core::domain::User>, apex_core::error::RepoError> {
        Ok(None)
    }
    async fn save(
        &self,
        u: apex_core::domain::User,
    ) -> Result<apex_core::domain::User, apex_core::error::RepoError> {
        Ok(u)
    }
    async fn delete(&self, _id: uuid::Uuid) -> Result<(), apex_core::error::RepoError> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl UserRepository for StubUserRepository {
    async fn find_by_email(
        &self,
        _email: &str,
    ) -> Result<Option<apex_core::domain::User>, apex_core::error::RepoError> {
        Ok(None)
    }
}

/// In-memory post repository (Stub)
pub struct StubPostRepository;
#[async_trait::async_trait]
impl apex_core::ports::BaseRepository<apex_core::domain::Post, uuid::Uuid> for StubPostRepository {
    async fn find_by_id(
        &self,
        _id: uuid::Uuid,
    ) -> Result<Option<apex_core::domain::Post>, apex_core::error::RepoError> {
        Ok(None)
    }
    async fn save(
        &self,
        p: apex_core::domain::Post,
    ) -> Result<apex_core::domain::Post, apex_core::error::RepoError> {
        Ok(p)
    }
    async fn delete(&self, _id: uuid::Uuid) -> Result<(), apex_core::error::RepoError> {
        Ok(())
    }
}
#[async_trait::async_trait]
impl PostRepository for StubPostRepository {
    async fn find_by_user_id(
        &self,
        _user_id: uuid::Uuid,
    ) -> Result<Vec<apex_core::domain::Post>, apex_core::error::RepoError> {
        Ok(vec![])
    }
}

impl AppState {
    /// Build the application state with appropriate implementations.
    pub async fn new(db_config: Option<&DatabaseConfig>) -> Self {
        // Initialize cache (in-memory for now, Redis later)
        let cache: Arc<dyn Cache> = Arc::new(InMemoryCache::new());

        // Initialize database connections if configured
        #[cfg(feature = "postgres")]
        let (db, users, posts): (
            Option<Arc<DatabaseConnections>>,
            Arc<dyn UserRepository>,
            Arc<dyn PostRepository>,
        ) = {
            if let Some(config) = db_config {
                match DatabaseConnections::init(config).await {
                    Ok(connections) => {
                        let conn = Arc::new(connections);
                        let user_repo = Arc::new(PostgresUserRepository::new(conn.main.clone()));
                        let post_repo = Arc::new(PostgresPostRepository::new(conn.main.clone()));
                        (Some(conn), user_repo, post_repo)
                    }
                    Err(e) => {
                        tracing::error!(
                            "Failed to connect to database: {}. Using stub fallback.",
                            e
                        );
                        (
                            None,
                            Arc::new(StubUserRepository),
                            Arc::new(StubPostRepository),
                        )
                    }
                }
            } else {
                tracing::warn!("DATABASE_URL not set. Running without database stub mode).");
                (
                    None,
                    Arc::new(StubUserRepository),
                    Arc::new(StubPostRepository),
                )
            }
        };

        #[cfg(not(feature = "postgres"))]
        let (db, users, posts): (
            Option<Arc<DatabaseConnections>>,
            Arc<dyn UserRepository>,
            Arc<dyn PostRepository>,
        ) = {
            tracing::info!("Running without postgres feature - using stub repository");
            (
                None,
                Arc::new(StubUserRepository),
                Arc::new(StubPostRepository),
            )
        };

        tracing::info!("Application state initialized");

        Self {
            cache,
            users,
            posts,
            db,
        }
    }
}
