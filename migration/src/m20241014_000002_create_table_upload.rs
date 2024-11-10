use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241014_000002_create_table_upload"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Upload::Table)
                    .col(ColumnDef::new(Upload::Uuid).uuid().primary_key())
                    .col(ColumnDef::new(Upload::Typ).string().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(User::Table, User::Avatar)
                    .to(Upload::Table, Upload::Uuid)
                    .on_delete(ForeignKeyAction::SetNull)
                    .on_update(ForeignKeyAction::SetNull)
                    .name("FK_USER_AVATAR_UPLOAD_UUID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_USER_AVATAR_UPLOAD_UUID")
                    .table(User::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Upload::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Upload {
    Table,
    Uuid,
    Typ,
}
