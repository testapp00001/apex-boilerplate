//! PostgreSQL implementation of the UserRepository trait.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbConn, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use apex_core::domain::User;
use apex_core::error::RepoError;
use apex_core::ports::UserRepository;

use super::entity::user::{self, Entity as UserEntity};

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
        tracing::debug!("Finding user by id: {}", id);

        let result = UserEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.map(Into::into))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepoError> {
        tracing::debug!("Finding user by email: {}", email);

        let result = UserEntity::find()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.map(Into::into))
    }

    async fn save(&self, user: User) -> Result<User, RepoError> {
        tracing::debug!("Saving user: {}", user.id);

        // Check if user exists
        let existing = UserEntity::find_by_id(user.id)
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        let model = if existing.is_some() {
            // Update existing user
            let active_model = user::ActiveModel {
                id: Set(user.id),
                email: Set(user.email.clone()),
                password_hash: Set(user.password_hash.clone()),
                created_at: Set(user.created_at.into()),
                updated_at: Set(chrono::Utc::now().into()),
            };

            active_model
                .update(&self.db)
                .await
                .map_err(|e| RepoError::Query(e.to_string()))?
        } else {
            // Insert new user
            let active_model = user::ActiveModel {
                id: Set(user.id),
                email: Set(user.email.clone()),
                password_hash: Set(user.password_hash.clone()),
                created_at: Set(user.created_at.into()),
                updated_at: Set(user.updated_at.into()),
            };

            active_model.insert(&self.db).await.map_err(|e| {
                if e.to_string().contains("duplicate") || e.to_string().contains("unique") {
                    RepoError::Constraint("Email already exists".to_string())
                } else {
                    RepoError::Query(e.to_string())
                }
            })?
        };

        Ok(model.into())
    }

    async fn delete(&self, id: Uuid) -> Result<(), RepoError> {
        tracing::debug!("Deleting user: {}", id);

        let result = UserEntity::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(RepoError::NotFound);
        }

        Ok(())
    }
}
