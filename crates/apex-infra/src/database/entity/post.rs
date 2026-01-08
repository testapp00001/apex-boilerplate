//! Post entity for SeaORM.

use sea_orm::Set;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    #[sea_orm(column_type = "Text")]
    pub content: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

/// Conversion from SeaORM Model to Domain Post.
impl From<Model> for apex_core::domain::Post {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            title: model.title,
            content: model.content,
            created_at: model.created_at.into(),
            updated_at: model.updated_at.into(),
        }
    }
}

/// Conversion from Domain Post to SeaORM ActiveModel.
impl From<apex_core::domain::Post> for ActiveModel {
    fn from(post: apex_core::domain::Post) -> Self {
        Self {
            id: Set(post.id),
            user_id: Set(post.user_id),
            title: Set(post.title),
            content: Set(post.content),
            created_at: Set(post.created_at.into()),
            updated_at: Set(post.updated_at.into()),
        }
    }
}
