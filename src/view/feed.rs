use super::*;

use entity::{
    feed, message,
    prelude::{Feed, Message},
};

use contact::ContactList;

/// 用户或群聊的状态更新
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct FeedItem {
    /// 用户或群聊的 UUID
    id: Uuid,
    /// 会话
    session: Uuid,
    /// 新消息计数
    cnt: u64,
}

/// 新消息通知
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Notification {
    /// 群聊新消息
    Groups { feeds: Vec<FeedItem> },
    /// 双人聊天新消息
    Chats { feeds: Vec<FeedItem> },
    /// 群聊新公告
    Notices { feeds: Vec<FeedItem> },
    /// 新的希望添加自己的联系人
    ContactRequests { items: ContactList },
    /// 新的被接受的联系人添加请求
    ContactAccepts { items: ContactList },
    /// 新的加入群组邀请
    GroupInvites { items: Vec<Uuid> },
    /// 新的希望加入自己管理的群组的请求
    GroupRequests { items: Vec<Uuid> },
}

async fn count_unread_msgs(
    user: Uuid,
    session: Uuid,
    notice: bool,
    conn: &DatabaseConnection,
) -> Result<u64, AppError> {
    Ok(Message::find()
        .join_rev(
            JoinType::InnerJoin,
            feed::Entity::belongs_to(message::Entity)
                .from(feed::Column::Message)
                .to(message::Column::Id)
                .into(),
        )
        .filter(feed::Column::User.eq(user))
        .filter(message::Column::Session.eq(session))
        .filter(message::Column::Notice.eq(notice))
        .filter(feed::Column::ReadAt.is_null())
        .count(conn)
        .await?)
}

impl FeedItem {
    pub(super) async fn from_group_sessions(
        group_sessions: &Vec<(Uuid, Uuid)>,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Vec<Self>, AppError> {
        let mut feeds = Vec::new();
        for (group, session) in group_sessions {
            let cnt = count_unread_msgs(user, *session, false, conn).await?;
            feeds.push(FeedItem {
                id: *group,
                session: *session,
                cnt,
            });
        }
        Ok(feeds)
    }

    pub(super) async fn from_chat_sessions(
        chat_sessions: &Vec<(Uuid, Uuid)>,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Vec<Self>, AppError> {
        let mut feeds = Vec::new();
        for (chat, session) in chat_sessions {
            let cnt = count_unread_msgs(user, *session, false, conn).await?;
            feeds.push(FeedItem {
                id: *chat,
                session: *session,
                cnt,
            });
        }
        Ok(feeds)
    }

    pub(super) async fn from_notice_sessions(
        notice_sessions: &Vec<(Uuid, Uuid)>,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Vec<Self>, AppError> {
        let mut feeds = Vec::new();
        for (notice, session) in notice_sessions {
            let cnt = count_unread_msgs(user, *session, true, conn).await?;
            feeds.push(FeedItem {
                id: *notice,
                session: *session,
                cnt,
            });
        }
        Ok(feeds)
    }
}

impl Feed {
    pub(super) async fn ack_msgs(
        user: Uuid,
        msgs: Vec<Uuid>,
        conn: &DatabaseConnection,
    ) -> Result<(), AppError> {
        for msg in msgs {
            Feed::update_many()
                .col_expr(feed::Column::ReadAt, chrono::Utc::now().naive_utc().into())
                .filter(feed::Column::User.eq(user))
                .filter(feed::Column::Message.eq(msg))
                .filter(feed::Column::ReadAt.is_null())
                .exec(conn)
                .await?;
        }
        Ok(())
    }
}
