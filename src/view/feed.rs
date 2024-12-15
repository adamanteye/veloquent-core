use super::*;

use entity::{
    feed, message,
    prelude::{Feed, Message},
};

use super::message::ReadAt;
use contact::ContactList;

/// 用户或群聊的消息更新
#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct FeedItem {
    /// 用户或群聊的 UUID
    id: Uuid,
    /// 会话
    session: Uuid,
    /// 新消息计数
    cnt: u64,
}

/// 群聊的用户更新
#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct GroupUpdate {
    pub group: Uuid,
    pub user: Uuid,
}

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
#[cfg_attr(test, derive(Deserialize))]
pub struct ReadMsg {
    pub msg: Uuid,
    pub read_ats: Vec<ReadAt>,
}

/// 新消息通知
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
#[serde(tag = "type")]
pub enum Notification {
    Reads {
        feeds: Vec<ReadMsg>,
    },
    /// 群聊新消息
    Groups {
        feeds: Vec<FeedItem>,
    },
    /// 双人聊天新消息
    Chats {
        feeds: Vec<FeedItem>,
    },
    /// 群聊新公告
    Notices {
        feeds: Vec<FeedItem>,
    },
    /// 新的希望添加自己的联系人
    ContactRequests {
        items: ContactList,
    },
    /// 新的被接受的联系人添加请求
    ContactAccepts {
        items: ContactList,
    },
    /// 新的加入群组邀请
    GroupInvites {
        items: Vec<GroupUpdate>,
    },
    /// 新的希望加入自己管理的群组的请求
    GroupRequests {
        items: Vec<GroupUpdate>,
    },
    /// 新的接受加入群组通知
    GroupAccepts {
        items: Vec<GroupUpdate>,
    },
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
    pub(super) async fn from_group(
        group: Uuid,
        session: Uuid,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let cnt = count_unread_msgs(user, session, false, conn).await?;
        Ok(FeedItem {
            id: group,
            session,
            cnt,
        })
    }

    pub(super) async fn from_chat(
        ref_user: Uuid,
        session: Uuid,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let cnt = count_unread_msgs(user, session, false, conn).await?;
        Ok(FeedItem {
            id: ref_user,
            session,
            cnt,
        })
    }

    pub(super) async fn from_notice(
        notice: Uuid,
        session: Uuid,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let cnt = count_unread_msgs(user, session, true, conn).await?;
        Ok(FeedItem {
            id: notice,
            session,
            cnt,
        })
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
