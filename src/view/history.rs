use super::message::{Msg, ReadAt, Reader};
use super::*;
use entity::{
    feed, message,
    prelude::{Feed, Message},
};

/// 聊天记录
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize, Debug)]
pub struct History {
    /// 消息列表
    pub msgs: Vec<Msg>,
    /// 起始位置
    ///
    /// 最小值为 `0`, 代表上一条消息
    pub start: u64,
    /// 结束位置
    ///
    /// 最大值为 `cnt`
    pub end: u64,
    /// 聊天消息总数
    ///
    /// 这代表服务器存储的消息总数
    pub cnt: u64,
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub struct HistoryRequest {
    /// 最近一条消息, 默认为 `0`
    start: Option<u64>,
    /// 最早一条消息, 默认为 `50`
    end: Option<u64>,
}

impl History {
    pub async fn find_by_session(
        req: HistoryRequest,
        conn: &DatabaseConnection,
        session: Uuid,
    ) -> Result<Self, AppError> {
        let start = req.start.unwrap_or(0);
        let end = req.end.unwrap_or(50);
        if end <= start {
            return Err(AppError::BadRequest("end leq start".to_string()));
        }
        let msgs = Message::find()
            .filter(message::Column::Session.eq(session))
            .order_by(message::Column::CreatedAt, sea_orm::Order::Desc)
            .limit(Some((end - start) as u64))
            .all(conn)
            .await?
            .split_off(start as usize);
        let mut read_ats = Vec::new();
        for msg in &msgs {
            let read_at: Vec<ReadAt> = Reader::fetch_from_db(msg.id, conn)
                .await?
                .into_iter()
                .map(ReadAt::from)
                .collect();
            read_ats.push(read_at);
        }
        let msgs: Vec<Msg> = msgs.into_iter().zip(read_ats).map(Msg::from).collect();
        let end = start + msgs.len() as u64;
        let cnt = Message::find()
            .filter(message::Column::Session.eq(session))
            .count(conn)
            .await?;
        Ok(History {
            msgs,
            start,
            end,
            cnt,
        })
    }
}

/// 获取历史聊天记录
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/msg/session/{id}",
    params(
        ("id" = Uuid, Path, description = "会话的唯一主键"),
        HistoryRequest
    ),
    responses(
        (status = 200, description = "获取成功", body = History),
    ),
    tag = "msg"
))]
#[instrument(skip(state))]
pub async fn get_history_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Query(params): Query<HistoryRequest>,
    Path(session): Path<Uuid>,
) -> Result<Json<History>, AppError> {
    let history = History::find_by_session(params, &state.conn, session).await?;
    event!(Level::DEBUG, "get history");
    Ok(Json(history))
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
pub async fn mask_history_handler(
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
