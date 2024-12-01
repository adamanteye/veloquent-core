use super::message::Msg;
use super::*;
use entity::{message, prelude::Message};

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
        let msgs: Vec<Msg> = msgs.into_iter().map(Msg::from).collect();
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

/// 获取聊天历史记录
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/msg/session/{id}",
    params(HistoryRequest),
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
