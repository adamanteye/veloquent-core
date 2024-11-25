use super::*;
use entity::{message, prelude::Message as MessageEntity};
use std::str::FromStr;

#[derive(prost::Message)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct MsgPost {
    /// 指示消息类型
    ///
    /// | 值 | 类型 |
    /// |----|----|
    /// | 0 | 文本 |
    /// | 1 | 图片 |
    /// | 2 | 文件 |
    ///
    /// `tag` = `1`
    #[prost(int32, tag = "1")]
    pub typ: i32,
    /// 消息内容
    ///
    /// 在消息的值为图片或文件时, 消息内容存储消息的文件名, 即需要先上传文件再发送消息
    ///
    /// `tag` = `2`
    #[prost(string, optional, tag = "2")]
    pub content: Option<String>,
    /// 引用消息的 UUID
    ///
    /// `tag` = `3`
    #[prost(string, optional, tag = "3")]
    pub cite: Option<String>,
    /// 消息的文件 UUID
    ///
    /// `tag` = `4`
    #[prost(string, optional, tag = "4")]
    pub file: Option<String>,
}

impl TryFrom<(MsgPost, Uuid, Uuid)> for message::ActiveModel {
    type Error = AppError;
    fn try_from(value: (MsgPost, Uuid, Uuid)) -> Result<Self, Self::Error> {
        let file = match value
            .0
            .file
            .map(|s| Uuid::from_str(&s).map_err(|e| AppError::BadRequest(format!("{}", e))))
        {
            Some(Ok(uuid)) => Some(uuid),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        let cite = match value
            .0
            .cite
            .map(|s| Uuid::from_str(&s).map_err(|e| AppError::BadRequest(format!("{}", e))))
        {
            Some(Ok(uuid)) => Some(uuid),
            Some(Err(e)) => return Err(e),
            None => None,
        };
        Ok(message::ActiveModel {
            id: ActiveValue::not_set(),
            created_at: ActiveValue::not_set(),
            edited_at: ActiveValue::not_set(),
            sender: ActiveValue::set(Some(value.1)),
            session: ActiveValue::set(value.2),
            content: ActiveValue::set(value.0.content),
            typ: ActiveValue::set(value.0.typ),
            file: ActiveValue::set(file),
            cite: ActiveValue::set(cite),
        })
    }
}

#[derive(prost::Message)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct MsgRes {
    /// 指示消息类型
    ///
    /// | 值 | 类型 |
    /// |----|----|
    /// | 0 | 文本 |
    /// | 1 | 图片 |
    /// | 2 | 文件 |
    ///
    /// `tag` = `1`
    #[prost(int32, tag = "1")]
    pub typ: i32,
    /// 消息 UUID
    ///
    /// `tag` = `2`
    #[prost(string, tag = "2")]
    pub id: String,
    /// UTC 毫秒时间戳
    ///
    /// `tag` = `3`
    #[prost(int64, tag = "3")]
    pub created_at: i64,
}

impl From<message::Model> for MsgRes {
    fn from(value: message::Model) -> Self {
        Self {
            typ: value.typ,
            id: value.id.to_string(),
            created_at: value.created_at.and_utc().timestamp_millis(),
        }
    }
}

/// 发送新消息
///
/// 返回 protobuf 格式的响应
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/msg/session/{id}",
    params(
        ("id" = String, Path, description = "会话的唯一主键")
    ),
    request_body = MessagePost,
    responses(
        (status = 200, description = "发送成功, 服务器成功存储", body = MessageResponse),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn send_msg_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(session): Path<Uuid>,
    Protobuf(msg): Protobuf<MsgPost>,
) -> Result<Protobuf<MsgRes>, AppError> {
    let msg: message::ActiveModel = (msg, payload.id, session).try_into()?;
    let res = MessageEntity::insert(msg).exec(&state.conn).await?;
    event!(Level::DEBUG, "send message: [{:?}]", res);
    let msg = MessageEntity::find_by_id(res.last_insert_id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::Server(anyhow::anyhow!("cannot store message")))?;
    Ok(Protobuf(msg.into()))
}
