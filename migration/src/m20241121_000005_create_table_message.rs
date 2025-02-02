use sea_orm_migration::prelude::*;

use super::m20241012_000001_create_table_user::User;
use super::m20241014_000002_create_table_upload::Upload;
use super::m20241110_000004_create_table_session::Session;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20241121_000005_create_table_message"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .col(
                        ColumnDef::new(Message::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT uuid_generate_v4()"),
                    )
                    .col(
                        ColumnDef::new(Message::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT now()::TIMESTAMP"),
                    )
                    .col(ColumnDef::new(Message::Session).uuid().not_null())
                    .col(ColumnDef::new(Message::EditedAt).timestamp())
                    .col(ColumnDef::new(Message::Content).string())
                    .col(ColumnDef::new(Message::Typ).integer().not_null())
                    .col(ColumnDef::new(Message::File).uuid())
                    .col(ColumnDef::new(Message::Sender).uuid())
                    .col(ColumnDef::new(Message::Cite).uuid())
                    .col(ColumnDef::new(Message::FwdVon).uuid())
                    .col(
                        ColumnDef::new(Message::Notice)
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
                    .from(Message::Table, Message::Session)
                    .to(Session::Table, Session::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_MESSAGE_SESSION_SESSION_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Message::Table, Message::Cite)
                    .to(Message::Table, Message::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .name("FK_MESSAGE_CITE_SESSION_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Message::Table, Message::FwdVon)
                    .to(Message::Table, Message::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .name("FK_MESSAGE_FWDVON_SESSION_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Message::Table, Message::Sender)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .name("FK_MESSAGE_SENDER_USER_ID")
                    .to_owned(),
            )
            .await?;
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .from(Message::Table, Message::File)
                    .to(Upload::Table, Upload::Uuid)
                    .on_delete(ForeignKeyAction::SetNull)
                    .name("FK_MESSAGE_FILE_UPLOAD_UUID")
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MESSAGE_FILE_UPLOAD_UUID")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MESSAGE_SESSION_SESSION_ID")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MESSAGE_FWDVON_SESSION_ID")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MESSAGE_CITE_SESSION_ID")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("FK_MESSAGE_SENDER_USER_ID")
                    .table(Message::Table)
                    .to_owned(),
            )
            .await?;
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
pub enum Message {
    Table,
    Id,
    Typ,
    Content,
    File,
    Cite,
    Sender,
    FwdVon,
    CreatedAt,
    EditedAt,
    Session,
    Notice,
}
