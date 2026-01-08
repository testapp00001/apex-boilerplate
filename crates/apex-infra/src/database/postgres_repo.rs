//! PostgreSQL repository implementations.

use async_trait::async_trait;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

use apex_core::domain::{Post, User};
use apex_core::error::RepoError;
use apex_core::ports::{PostRepository, UserRepository};

use super::entity::post::{self, Entity as PostEntity};
use super::entity::user::{self, Entity as UserEntity};
use super::postgres_base::PostgresBaseRepository;

/// PostgreSQL user repository.
pub type PostgresUserRepository = PostgresBaseRepository<UserEntity>;

/// PostgreSQL post repository.
pub type PostgresPostRepository = PostgresBaseRepository<PostEntity>;

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        // Mask email for logging to avoid PII in logs
        let masked = if let Some(at_pos) = email.find('@') {
            let (local, domain) = email.split_at(at_pos);
            let masked_local = if local.len() > 1 {
                format!("{}***", &local[..1])
            } else {
                "***".to_string()
            };
            format!("{}{}", masked_local, domain)
        } else {
            "***".to_string()
        };
        tracing::debug!(user_email = %masked, "Finding user by email");

        let result = UserEntity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.map(Into::into))
    }
}

#[async_trait]
impl PostRepository for PostgresPostRepository {
    async fn find_by_user_id(&self, user_id: uuid::Uuid) -> Result<Vec<Post>, RepoError> {
        let result = PostEntity::find()
            .filter(post::Column::UserId.eq(user_id))
            .all(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.into_iter().map(Into::into).collect())
    }
}
