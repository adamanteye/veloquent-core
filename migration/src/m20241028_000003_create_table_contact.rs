use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241028_000003_create_table_contact"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Contact::Table)
                    .col(
                        ColumnDef::new(Contact::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(Contact::User).uuid().not_null())
                    .col(ColumnDef::new(Contact::RefUser).uuid())
                    .col(ColumnDef::new(Contact::Category).string())
                    .col(ColumnDef::new(Contact::Chat).uuid().not_null())
                    .col(ColumnDef::new(Contact::Alias).string())
                    .col(
                        ColumnDef::new(Contact::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT now()::TIMESTAMP"),
                    )
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Contact::Table, Contact::User)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_CONTACT_USER_USER_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Contact::Table, Contact::RefUser)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .name("FK_CONTACT_REF_USER_ID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_CONTACT_USER_USER_ID")
                    .table(Contact::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_CONTACT_REF_USER_ID")
                    .table(Contact::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Contact::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Contact {
    Table,
    Id,
    User,
    RefUser,
    Alias,
    Category,
    Chat,
    CreatedAt,
}
