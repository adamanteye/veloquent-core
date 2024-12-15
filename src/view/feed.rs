use super::*;

use entity::{feed, prelude::Feed};

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
    /// 发送者
    sender: Uuid,
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

impl FeedItem {
    pub(super) fn from_group(group: Uuid, session: Uuid, sender: Uuid) -> Result<Self, AppError> {
        Ok(FeedItem {
            id: group,
            session,
            sender,
        })
    }

    pub(super) fn from_chat(ref_user: Uuid, session: Uuid, sender: Uuid) -> Result<Self, AppError> {
        Ok(FeedItem {
            id: ref_user,
            session,
            sender,
        })
    }

    pub(super) fn from_notice(notice: Uuid, session: Uuid, sender: Uuid) -> Result<Self, AppError> {
        Ok(FeedItem {
            id: notice,
            session,
            sender,
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
