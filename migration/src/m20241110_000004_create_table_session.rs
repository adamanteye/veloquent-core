use sea_orm_migration::prelude::*;

use super::m20241028_000003_create_table_contact::Contact;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241110_000004_create_table_session"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Session::Table)
                    .col(
                        ColumnDef::new(Session::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v4()"),
                    )
                    .col(
                        ColumnDef::new(Session::CreatedAt)
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
                    .from(Contact::Table, Contact::Session)
                    .to(Session::Table, Session::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_CONTACT_SESSION_SESSION_ID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_CONTACT_SESSION_SESSION_ID")
                    .table(Contact::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Session::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Session {
    Table,
    Id,
    CreatedAt,
}
