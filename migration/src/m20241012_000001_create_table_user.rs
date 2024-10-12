use sea_orm_migration::prelude::*;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m_20220602_000001_create_bakery_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // Define how to apply this migration: Create the Bakery table.
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .col(
                        ColumnDef::new(User::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(User::Name).string().not_null().unique_key())
                    .col(ColumnDef::new(User::Alias).string())
                    .col(ColumnDef::new(User::Salt).string().not_null())
                    .col(ColumnDef::new(User::Hash).string().not_null())
                    .col(ColumnDef::new(User::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(User::Gender).integer())
                    .col(ColumnDef::new(User::Email).string().unique_key())
                    .col(ColumnDef::new(User::Phone).string().unique_key())
                    .to_owned(),
            )
            .await
    }

    // Define how to rollback this migration: Drop the Bakery table.
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
}
