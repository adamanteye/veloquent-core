//! `SeaORM` Entity, @generated by sea-orm-codegen 1.1.1

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "contact")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub user: Uuid,
    pub ref_user: Option<Uuid>,
    pub category: Option<String>,
    pub chat: Uuid,
    pub alias: Option<String>,
    pub created_at: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::session::Entity",
        from = "Column::Chat",
        to = "super::session::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    Session,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::RefUser",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "SetNull"
    )]
    User2,
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::User",
        to = "super::user::Column::Id",
        on_update = "NoAction",
        on_delete = "Cascade"
    )]
    User1,
}

impl Related<super::session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Session.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
