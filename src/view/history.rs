use super::message::Msg;
use super::*;

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct History {
    /// 消息列表
    pub users: Vec<Msg>,
    /// 起始位置
    ///
    /// 最小值为 `0`, 代表上一条消息
    pub start: i32,
    /// 结束位置
    ///
    /// 最大值为 `cnt`
    pub end: i32,
    /// 聊天消息总数
    ///
    /// 这代表服务器存储的消息总数
    pub cnt: i32,
}

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct HistoryRequest {
    /// 最近一条消息, 默认为 `0`
    pub start: Option<i32>,
    /// 最早一条消息, 默认为 `50`
    pub end: Option<i32>,
    /// 会话 UUID
    pub session: Uuid,
}
