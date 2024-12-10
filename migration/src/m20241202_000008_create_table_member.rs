use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;
use super::m20241121_000007_create_table_group::Group;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241202_000008_create_table_member"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Member::Table)
                    .col(
                        ColumnDef::new(Member::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Member::CreatedAt).timestamp())
                    .col(ColumnDef::new(Member::User).uuid().not_null())
                    .col(ColumnDef::new(Member::Group).uuid().not_null())
                    .col(ColumnDef::new(Member::Permission).integer().not_null())
                    .col(
                        ColumnDef::new(Member::Pin)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(
                        ColumnDef::new(Member::Mute)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Member::Table, Member::User)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_MEMBER_USER_USER_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Member::Table, Member::Group)
                    .to(Group::Table, Group::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_MEMBER_GROUP_GROUP_ID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MEMBER_GROUP_GROUP_ID")
                    .table(Group::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MEMBER_USER_USER_ID")
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
pub enum Member {
    Table,
    Id,
    User,
    Group,
    CreatedAt,
    Permission,
    Pin,
    Mute,
}
