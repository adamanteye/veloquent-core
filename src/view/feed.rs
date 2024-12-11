use super::contact::Chat;
use super::*;

use entity::{feed, prelude::Feed};

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
pub struct Notification {
    /// 群聊新消息
    groups: Vec<FeedItem>,
    /// 双人聊天新消息
    chats: Vec<FeedItem>,
    /// 新的希望添加自己的联系人
    contact_requests: Vec<Chat>,
    /// 新的被接受的联系人添加请求
    contact_accepts: Vec<Chat>,
    /// 新的加入群组邀请
    group_invites: Vec<Chat>,
    /// 新的希望加入自己管理的群组的请求
    group_requests: Vec<Chat>,
    /// 总计数, 包含新消息和新联系人/群聊申请
    cnt: u64,
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
