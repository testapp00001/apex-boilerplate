//! PostgreSQL implementation of the UserRepository trait.

use async_trait::async_trait;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbConn, DbErr, EntityTrait, QueryFilter, Set};
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

    /// Convert SeaORM DbErr to RepoError with proper constraint detection.
    fn map_db_error(e: DbErr) -> RepoError {
        match &e {
            DbErr::Query(runtime_err) => {
                let err_str = runtime_err.to_string();
                if err_str.contains("duplicate") || err_str.contains("unique") {
                    RepoError::Constraint("Email already exists".to_string())
                } else {
                    RepoError::Query(e.to_string())
                }
            }
            DbErr::Exec(runtime_err) => {
                let err_str = runtime_err.to_string();
                if err_str.contains("duplicate") || err_str.contains("unique") {
                    RepoError::Constraint("Email already exists".to_string())
                } else {
                    RepoError::Query(e.to_string())
                }
            }
            _ => RepoError::Query(e.to_string()),
        }
    }
}

#[async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, RepoError> {
        tracing::debug!(user_id = %id, "Finding user by id");

        let result = UserEntity::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.map(Into::into))
    }

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

    async fn save(&self, user: User) -> Result<User, RepoError> {
        tracing::debug!(user_id = %user.id, "Saving user");

        let now = chrono::Utc::now();

        // Use insert-first upsert pattern: attempt insert, fallback to update on conflict
        // This avoids the N+1 query pattern of first checking if user exists
        let active_model = user::ActiveModel {
            id: Set(user.id),
            email: Set(user.email.clone()),
            password_hash: Set(user.password_hash.clone()),
            created_at: Set(user.created_at.into()),
            updated_at: Set(now.into()),
        };

        // Try insert first - most common case for new users
        let result = active_model.insert(&self.db).await;

        let model = match result {
            Ok(m) => m,
            Err(e) => {
                // Check if it's a duplicate/unique constraint violation (user exists)
                let is_conflict = match &e {
                    DbErr::Query(re) | DbErr::Exec(re) => {
                        let s = re.to_string();
                        s.contains("duplicate") || s.contains("unique")
                    }
                    _ => false,
                };

                if is_conflict {
                    // User exists, perform update instead
                    let update_model = user::ActiveModel {
                        id: Set(user.id),
                        email: Set(user.email.clone()),
                        password_hash: Set(user.password_hash.clone()),
                        created_at: Set(user.created_at.into()),
                        updated_at: Set(now.into()),
                    };
                    update_model
                        .update(&self.db)
                        .await
                        .map_err(Self::map_db_error)?
                } else {
                    return Err(Self::map_db_error(e));
                }
            }
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
