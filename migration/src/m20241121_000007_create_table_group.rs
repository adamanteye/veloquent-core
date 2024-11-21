use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;
use super::m20241110_000004_create_table_session::Session;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241121_000007_create_table_group"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Group::Table)
                    .col(
                        ColumnDef::new(Group::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(
                        ColumnDef::new(Group::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT now()::TIMESTAMP"),
                    )
                    .col(ColumnDef::new(Group::Owner).uuid().not_null())
                    .col(ColumnDef::new(Group::Session).uuid().not_null())
                    .col(ColumnDef::new(Group::Name).string())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Group::Table, Group::Owner)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_GROUP_OWNER_USER_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Group::Table, Group::Session)
                    .to(Session::Table, Session::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_GROUP_SESSION_SESSION_ID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_GROUP_SESSION_SESSION_ID")
                    .table(Group::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_GROUP_OWNER_USER_ID")
                    .table(Group::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Group::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Group {
    Table,
    Id,
    Name,
    CreatedAt,
    Owner,
    Session,
}
