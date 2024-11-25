use super::*;

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(prost::Message)]
pub struct History {
    /// 消息主键列表
    ///
    /// `tag` = `1`
    #[prost(message, repeated, tag = "1")]
    pub users: Vec<String>,
    /// 起始位置
    ///
    /// 最小值为 `0`, 代表上一条消息
    ///
    /// `tag` = `2`
    #[prost(int32, tag = "2")]
    pub start: i32,
    /// 结束位置
    ///
    /// 最大值为 `cnt`
    ///
    /// `tag` = `3`
    #[prost(int32, tag = "3")]
    pub end: i32,
    /// 消息总数
    ///
    /// `tag` = `4`
    #[prost(int32, tag = "4")]
    pub cnt: i32,
}
