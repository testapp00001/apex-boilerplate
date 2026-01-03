//! PostgreSQL implementation of the UserRepository trait.

use async_trait::async_trait;
use sea_orm::DbConn;
use uuid::Uuid;

use apex_core::domain::User;
use apex_core::error::RepoError;
use apex_core::ports::UserRepository;

/// PostgreSQL-backed user repository.
pub struct PostgresUserRepository {
    db: DbConn,
}

impl PostgresUserRepository {
    pub fn new(db: DbConn) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, RepoError> {
        // TODO: Implement with SeaORM entity queries
        // For now, return None as placeholder
        tracing::debug!("Finding user by id: {}", id);
        Ok(None)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        // TODO: Implement with SeaORM entity queries
        tracing::debug!("Finding user by email: {}", email);
        Ok(None)
    }

    async fn save(&self, user: User) -> Result<User, RepoError> {
        // TODO: Implement with SeaORM entity insert/update
        tracing::debug!("Saving user: {}", user.id);
        Ok(user)
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepoError> {
        // TODO: Implement with SeaORM entity delete
        tracing::debug!("Deleting user: {}", id);
        Ok(())
    }
}
