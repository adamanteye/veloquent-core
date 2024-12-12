use super::*;
use entity::{
    contact, feed, group, member, message,
    prelude::{Contact, Feed, Member, Message},
};

#[derive(Deserialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct MsgPost {
    /// 指示消息类型
    ///
    /// | 值 | 类型 |
    /// |----|----|
    /// | 0 | 文本 |
    /// | 1 | 图片 |
    /// | 2 | 文件 |
    typ: i32,
    /// 消息内容
    ///
    /// 在消息的值为图片或文件时, 消息内容存储消息的文件名, 即需要先上传文件再发送消息
    content: Option<String>,
    /// 引用消息的 UUID
    cite: Option<Uuid>,
    /// 消息的文件 UUID
    file: Option<Uuid>,
    /// 转发消息的 UUID
    forward: Option<Uuid>,
    /// 是否为通知
    notice: Option<bool>,
}

impl TryFrom<(MsgPost, Uuid, Uuid)> for message::ActiveModel {
    type Error = AppError;
    fn try_from(value: (MsgPost, Uuid, Uuid)) -> Result<Self, Self::Error> {
        let file = value
            .0
            .file
            .map(|u| ActiveValue::set(Some(u)))
            .unwrap_or(ActiveValue::not_set());
        let cite = value
            .0
            .cite
            .map(|u| ActiveValue::set(Some(u)))
            .unwrap_or(ActiveValue::not_set());
        let notice = value
            .0
            .notice
            .map(|b| ActiveValue::set(b))
            .unwrap_or(ActiveValue::not_set());
        let mut m = message::ActiveModel {
            id: ActiveValue::not_set(),
            created_at: ActiveValue::not_set(),
            edited_at: ActiveValue::not_set(),
            sender: ActiveValue::set(Some(value.1)),
            session: ActiveValue::set(value.2),
            content: ActiveValue::set(value.0.content),
            typ: ActiveValue::set(value.0.typ),
            file,
            cite,
            fwd_von: ActiveValue::not_set(),
            notice,
        };
        if let Some(f) = value.0.forward {
            m.fwd_von = ActiveValue::set(Some(f));
            m.content = ActiveValue::not_set();
            m.typ = ActiveValue::not_set();
            m.file = ActiveValue::not_set();
            m.cite = ActiveValue::not_set();
        }
        Ok(m)
    }
}

#[derive(Serialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
/// 消息
pub struct Msg {
    /// 消息 UUID
    id: Uuid,
    /// 创建时间戳, UTC 毫秒
    created_at: i64,
    /// 修改时间戳, UTC 毫秒
    edited_at: Option<i64>,
    /// 阅读者及阅读时间
    read_ats: Vec<ReadAt>,
    /// 发送者 UUID
    sender: Option<Uuid>,
    /// 引用消息的 UUID
    cite: Option<Uuid>,
    /// 消息类型
    typ: i32,
    /// 消息内容
    content: Option<String>,
    /// 文件 UUID
    file: Option<Uuid>,
    /// 所属会话
    session: Uuid,
}

#[derive(Serialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct ReadAt {
    /// 阅读者 UUID
    reader: Uuid,
    /// 阅读时间戳, UTC 毫秒
    read_at: i64,
}

impl From<(message::Model, Vec<ReadAt>)> for Msg {
    fn from(value: (message::Model, Vec<ReadAt>)) -> Self {
        Msg {
            id: value.0.id,
            created_at: value.0.created_at.and_utc().timestamp_millis(),
            edited_at: value
                .0
                .edited_at
                .and_then(|t| Some(t.and_utc().timestamp_millis())),
            typ: value.0.typ,
            content: value.0.content,
            file: value.0.file,
            sender: value.0.sender,
            cite: value.0.cite,
            read_ats: value.1,
            session: value.0.session,
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
/// 消息响应体
///
/// 成功发送消息后, 服务器返回的消息
pub struct MsgRes {
    /// 指示消息类型
    ///
    /// | 值 | 类型 |
    /// |----|----|
    /// | 0 | 文本 |
    /// | 1 | 图片 |
    /// | 2 | 文件 |
    pub typ: i32,
    /// 消息 UUID
    pub id: Uuid,
    /// UTC 毫秒时间戳
    pub created_at: i64,
}

impl From<message::Model> for MsgRes {
    fn from(value: message::Model) -> Self {
        Self {
            typ: value.typ,
            id: value.id,
            created_at: value.created_at.and_utc().timestamp_millis(),
        }
    }
}

#[derive(Debug, FromQueryResult)]
pub(super) struct Reader {
    reader: Uuid,
    read_at: Option<chrono::NaiveDateTime>,
}

impl Reader {
    pub(super) async fn fetch_from_db(
        id: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Vec<Self>, AppError> {
        Ok(Self::find_by_statement(Statement::from_sql_and_values(
            Postgres,
            "SELECT feed.user AS reader, feed.read_at FROM message INNER JOIN feed ON message.id = feed.message WHERE message.id = $1",
            [id.into()],
        ))
        .all(conn)
        .await?)
    }
}

impl From<Reader> for ReadAt {
    fn from(value: Reader) -> Self {
        Self {
            reader: value.reader,
            read_at: value
                .read_at
                .map(|t| t.and_utc().timestamp_millis())
                .unwrap_or(0),
        }
    }
}

/// 获取单条消息
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/msg/{id}",
    params(
        ("id" = Uuid, Path, description = "消息的唯一主键")
    ),
    responses(
        (status = 200, description = "获取成功", body = Msg),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn get_msg_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<Json<Msg>, AppError> {
    let msg: message::Model = Message::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find message [{}]", id)))?;
    let read_ats: Vec<ReadAt> = Reader::fetch_from_db(id, &state.conn)
        .await?
        .into_iter()
        .map(ReadAt::from)
        .collect();
    Ok(Json((msg, read_ats).into()))
}

/// 撤回消息
///
/// 撤回消息意味着删除, 数据库中不会保留消息的任何记录
#[cfg_attr(feature = "dev",
utoipa::path(
    delete,
    path = "/msg/{id}",
    params(
        ("id" = Uuid, Path, description = "消息的唯一主键")
    ),
    responses(
        (status = 204, description = "删除成功", body = Msg),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn delete_msg_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let _ = Message::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find message [{}]", id)))?;
    let res = Message::delete_by_id(id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        Err(AppError::NotFound(format!("cannot find message [{}]", id)))
    } else {
        event!(Level::DEBUG, "delete message [{}]", id);
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}

/// 发送新消息
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/msg/session/{id}",
    params(
        ("id" = Uuid, Path, description = "会话的唯一主键")
    ),
    request_body = MsgPost,
    responses(
        (status = 200, description = "发送成功, 服务器成功存储", body = MsgRes),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn send_msg_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(session): Path<Uuid>,
    Json(msg): Json<MsgPost>,
) -> Result<Json<MsgRes>, AppError> {
    let msg: message::ActiveModel = (msg, payload.id, session).try_into()?;
    if msg.notice == ActiveValue::set(true) {
        let g = group::Model::from_session(session, &state.conn).await?;
        let is_admin = Member::is_admin(g.id, payload.id, &state.conn).await?;
        if g.owner != payload.id && !is_admin {
            return Err(AppError::Forbidden("cannot send notice message".into()));
        }
    }
    let res = Message::insert(msg).exec(&state.conn).await?;
    let msg = Message::find_by_id(res.last_insert_id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::Server(anyhow::anyhow!("cannot store message")))?;
    tokio::task::spawn(async move {
        match Contact::find()
            .filter(contact::Column::Session.eq(session))
            .all(&state.conn)
            .await
        {
            Ok(contacts) => {
                let mut users: Vec<Uuid> = contacts.iter().map(|c| c.user).collect();
                users.sort();
                users.dedup();
                for user in users {
                    match Feed::insert(feed::ActiveModel::from((user, msg.id)))
                        .exec(&state.conn)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            event!(Level::ERROR, "cannot store contact feed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                event!(Level::ERROR, "cannot find contacts: {}", e);
            }
        }
        match Member::find()
            .join_rev(
                JoinType::InnerJoin,
                group::Entity::belongs_to(member::Entity)
                    .from(group::Column::Id)
                    .to(member::Column::Group)
                    .into(),
            )
            .filter(group::Column::Session.eq(session))
            .all(&state.conn)
            .await
        {
            Ok(group_users) => {
                let mut group_users: Vec<Uuid> = group_users.into_iter().map(|m| m.user).collect();
                group_users.sort();
                group_users.dedup();
                for user in group_users {
                    match Feed::insert(feed::ActiveModel::from((user, msg.id)))
                        .exec(&state.conn)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => {
                            event!(Level::ERROR, "cannot store group feed: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                event!(Level::ERROR, "cannot find group users: {}", e);
            }
        }
    });
    event!(
        Level::DEBUG,
        "new message [{}] by user [{}]",
        msg.id,
        payload.id
    );
    Ok(Json(msg.into()))
}

impl From<(Uuid, Uuid)> for feed::ActiveModel {
    fn from(value: (Uuid, Uuid)) -> Self {
        feed::ActiveModel {
            id: ActiveValue::not_set(),
            user: ActiveValue::set(value.0),
            message: ActiveValue::set(value.1),
            read_at: ActiveValue::not_set(),
        }
    }
}

/// 删除聊天记录
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/msg/mask/{id}",
    params(
        ("id" = Uuid, Path, description = "消息的唯一主键")
    ),
    responses(
        (status = 204, description = "删除成功"),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn mask_msg_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(msg): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let msg = Feed::find()
        .filter(feed::Column::Message.eq(msg))
        .filter(feed::Column::User.eq(payload.id))
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find message [{}]", msg)))?;
    let res = Feed::delete_by_id(msg.id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::Server(anyhow::anyhow!(
            "cannot mask message [{}]",
            msg.id
        )));
    }
    event!(Level::DEBUG, "mask message [{}]", msg.id);
    Ok(StatusCode::NO_CONTENT.into_response())
}
