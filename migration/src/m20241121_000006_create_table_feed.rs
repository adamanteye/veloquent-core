use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;
use super::m20241121_000005_create_table_message::Message;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241121_000006_create_table_feed"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Feed::Table)
                    .col(
                        ColumnDef::new(Feed::Id)
                            .integer()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Feed::ReadAt).timestamp())
                    .col(ColumnDef::new(Feed::User).uuid().not_null())
                    .col(ColumnDef::new(Feed::Message).uuid().not_null())
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Feed::Table, Feed::Message)
                    .to(Message::Table, Message::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_FEED_MESSAGE_MESSAGE_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Feed::Table, Feed::User)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_FEED_USER_USER_ID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_FEED_MESSAGE_MESSAGE_ID")
                    .table(Feed::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_FEED_USER_USER_ID")
                    .table(Feed::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Feed::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Feed {
    Table,
    Id,
    User,
    Message,
    ReadAt,
}
