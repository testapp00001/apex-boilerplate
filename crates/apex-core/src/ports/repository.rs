use async_trait::async_trait;
use uuid::Uuid;

use crate::domain::User;
use crate::error::RepoError;

/// User repository trait - defines data access operations for users.
/// Infrastructure layer provides the concrete implementation.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Find a user by their unique ID.
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, RepoError>;

    /// Find a user by their email address.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError>;

    /// Save a new user or update an existing one.
    async fn save(&self, user: User) -> Result<User, RepoError>;

    /// Delete a user by their ID.
    async fn delete(&self, id: Uuid) -> Result<(), RepoError>;
}
