use std::marker::PhantomData;

use async_trait::async_trait;
use sea_orm::{
    ActiveModelTrait, DbConn, EntityTrait, IntoActiveModel, PrimaryKeyTrait, TryIntoModel,
};

use apex_core::error::RepoError;
use apex_core::ports::BaseRepository;

/// Generic PostgreSQL repository implementation.
pub struct PostgresBaseRepository<E>
where
    E: EntityTrait,
{
    pub(crate) db: DbConn,
    _entity: PhantomData<E>,
}

impl<E> PostgresBaseRepository<E>
where
    E: EntityTrait,
{
    pub fn new(db: DbConn) -> Self {
        Self {
            db,
            _entity: PhantomData,
        }
    }
}

#[async_trait]
impl<E, T, ID> BaseRepository<T, ID> for PostgresBaseRepository<E>
where
    E: EntityTrait,
    E::Model: IntoActiveModel<E::ActiveModel> + Sync + Send,
    E::ActiveModel: ActiveModelTrait<Entity = E> + TryIntoModel<E::Model> + Send + Sync,
    E::PrimaryKey: PrimaryKeyTrait<ValueType = ID>,
    ID: Send + Sync + Into<sea_orm::Value> + Clone + Copy + 'static,
    T: From<E::Model> + Into<E::ActiveModel> + Send + Sync + 'static,
{
    async fn find_by_id(&self, id: ID) -> Result<Option<T>, RepoError> {
        let result = E::find_by_id(id)
            .one(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        Ok(result.map(Into::into))
    }

    async fn save(&self, entity: T) -> Result<T, RepoError> {
        // but for Postgres we can use OnConflict.

        let active_model: E::ActiveModel = entity.into();
        let result = active_model.save(&self.db).await.map_err(|e| {
            let err_str = e.to_string();
            if err_str.contains("duplicate") || err_str.contains("unique") {
                RepoError::Constraint("Entity already exists".to_string())
            } else {
                RepoError::Query(err_str)
            }
        })?;

        // We need to convert back to ActiveModel to get the Model to get T
        // Wait, save returns ActiveModel.
        // And we need to convert ActiveModel back to Model?
        // SeaORM ActiveModel has try_into_model()?

        let model = result
            .try_into_model()
            .map_err(|e| RepoError::Query(e.to_string()))?;
        Ok(model.into())
    }

    async fn delete(&self, id: ID) -> Result<(), RepoError> {
        let result = E::delete_by_id(id)
            .exec(&self.db)
            .await
            .map_err(|e| RepoError::Query(e.to_string()))?;

        if result.rows_affected == 0 {
            return Err(RepoError::NotFound);
        }

        Ok(())
    }
}
