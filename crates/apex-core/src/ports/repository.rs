use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::{Post, User};
use crate::error::RepoError;

/// Generic repository trait defining standard CRUD operations.
#[async_trait]
pub trait BaseRepository<T, ID>: Send + Sync {
    /// Find an entity by its unique ID.
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepoError>;

    /// Save an entity (create or update).
    async fn save(&self, entity: T) -> Result<T, RepoError>;

    /// Delete an entity by its ID.
    async fn delete(&self, id: ID) -> Result<(), RepoError>;
}

/// User repository with domain-specific methods.
#[async_trait]
pub trait UserRepository: BaseRepository<User, Uuid> {
    /// Find a user by their email address.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError>;
}

/// Post repository.
#[async_trait]
pub trait PostRepository: BaseRepository<Post, Uuid> {
    // Add specific methods here if needed (e.g., find_by_user_id)
    async fn find_by_user_id(&self, user_id: Uuid) -> Result<Vec<Post>, RepoError>;
}
