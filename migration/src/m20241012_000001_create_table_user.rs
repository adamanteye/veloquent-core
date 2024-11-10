use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241012_000001_create_table_user"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()"),
                    )
                    .col(ColumnDef::new(User::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Alias).string())
                    .col(ColumnDef::new(User::Salt).string().not_null())
                    .col(ColumnDef::new(User::Hash).string().not_null())
                    .col(
                        ColumnDef::new(User::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT now()::TIMESTAMP"),
                    )
                    .col(ColumnDef::new(User::Gender).integer().not_null())
                    .col(ColumnDef::new(User::Email).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Phone).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Avatar).uuid())
                    .col(ColumnDef::new(User::Bio).string())
                    .col(ColumnDef::new(User::Link).string())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
    Name,
    Alias,
    Salt,
    Hash,
    CreatedAt,
    Gender,
    Email,
    Phone,
    Bio,
    Link,
    Avatar,
}
