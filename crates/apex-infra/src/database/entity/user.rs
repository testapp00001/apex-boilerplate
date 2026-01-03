//! User entity for SeaORM.

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Conversion from SeaORM Model to Domain User.
impl From<Model> for apex_core::domain::User {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            email: model.email,
            password_hash: model.password_hash,
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }
}
