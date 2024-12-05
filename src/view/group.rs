use super::*;
use entity::{
    group, member,
    prelude::{Group, Member, Session, User},
    session,
};

/// 新建群聊请求
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Deserialize, Debug)]
pub struct GroupPost {
    /// 群名
    name: Option<String>,
    /// 群成员, 自动包含创建者, 此外最少需要一个成员
    members: Vec<Uuid>,
}

/// 群聊基本信息
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct GroupProfile {
    /// 群名
    name: Option<String>,
    /// 群主 UUID
    owner: Uuid,
    /// 群聊唯一主键
    id: Uuid,
    /// 群聊会话 UUID
    session: Uuid,
    /// 创建时间, ms 时间戳
    created_at: i64,
    /// 群成员 UUID
    ///
    /// 包含群主
    members: Vec<Uuid>,
    /// 管理员 UUID
    ///
    /// 不包含群主
    admins: Vec<Uuid>,
    /// 是否置顶
    pin: bool,
}

/// 获取群聊信息
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/group/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键")),
    responses(
        (status = 200, description = "获取成功", body = GroupProfile),
    ),
    tag = "group"
))]
pub async fn get_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<Json<GroupProfile>, AppError> {
    let g = GroupProfile::from_group_id(id, &state.conn, payload.id).await?;
    Ok(Json(g))
}

impl GroupProfile {
    async fn from_group_id(
        id: Uuid,
        conn: &DatabaseConnection,
        user: Uuid,
    ) -> Result<Self, AppError> {
        let g = Group::find_by_id(id)
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!("cannot find group [{id}]")))?;
        let members = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::Permission.eq(0))
            .all(conn)
            .await?;
        let mut pin = false;
        let members: Vec<Uuid> = members
            .into_iter()
            .map(|m| {
                if m.user == user {
                    pin = m.anheften;
                }
                m.user
            })
            .collect();
        let admins = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::Permission.eq(1))
            .all(conn)
            .await?;
        let admins = admins.into_iter().map(|m| m.user).collect();
        Ok(GroupProfile {
            name: g.name,
            owner: g.owner,
            id: g.id,
            session: g.session,
            created_at: g.created_at.and_utc().timestamp_millis(),
            members,
            admins,
            pin,
        })
    }
}

/// 列出用户所在的群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/group/list",
    responses(
        (status = 200, description = "成功获取", body = Vec<GroupProfile>),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn list_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<Vec<GroupProfile>>, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let groups = Member::find()
        .filter(member::Column::User.eq(user.id))
        .all(&state.conn)
        .await?;
    let mut g = Vec::new();
    for m in groups {
        let p = GroupProfile::from_group_id(m.group, &state.conn, payload.id).await?;
        g.push(p);
    }
    Ok(Json(g))
}

/// 创建群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/group/new",
    request_body = GroupPost,
    responses(
        (status = 200, description = "成功获取", body = GroupProfile),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn create_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Json(req): Json<GroupPost>,
) -> Result<Json<GroupProfile>, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let mut members = req.members;
    members.push(user.id);
    members.sort();
    members.dedup();
    if members.len() < 2 {
        return Err(AppError::BadRequest("at least 2 members".to_string()));
    }
    let s = session::ActiveModel {
        id: ActiveValue::not_set(),
        created_at: ActiveValue::not_set(),
    };
    let s = Session::insert(s).exec(&state.conn).await?.last_insert_id;
    let g = group::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set(req.name),
        owner: ActiveValue::set(user.id),
        session: ActiveValue::set(s),
        created_at: ActiveValue::not_set(),
    };
    let g = Group::insert(g).exec(&state.conn).await?.last_insert_id;
    for m in members {
        let m = member::ActiveModel {
            id: ActiveValue::not_set(),
            group: ActiveValue::set(g),
            user: ActiveValue::set(m),
            permission: ActiveValue::set(0),
            created_at: ActiveValue::not_set(),
            anheften: ActiveValue::set(false),
        };
        Member::insert(m).exec(&state.conn).await?;
    }
    let g = GroupProfile::from_group_id(g, &state.conn, payload.id).await?;
    Ok(Json(g))
}

/// 邀请加入群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/group/invite/{id}",
    request_body = Vec<Uuid>,
    params(("id" = Uuid, Path, description = "群聊的唯一主键")),
    responses(
        (status = 200, description = "成功邀请"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn invite_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
    Json(users): Json<Vec<Uuid>>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let g = Group::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find group [{}]", id)))?;
    let _ = Member::find()
        .filter(member::Column::Group.eq(g.id))
        .filter(member::Column::User.eq(user.id))
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("inviter not in group [{}]", id)))?;
    for u in users.into_iter() {
        let _ = User::find_by_id(u)
            .one(&state.conn)
            .await?
            .ok_or(AppError::NotFound(format!("cannot find user [{}]", u)))?;
        let m = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::User.eq(u))
            .one(&state.conn)
            .await?;
        if m.is_some() {
            continue;
        }
        let m = member::ActiveModel {
            id: ActiveValue::not_set(),
            group: ActiveValue::set(g.id),
            user: ActiveValue::set(u),
            permission: ActiveValue::set(-1),
            created_at: ActiveValue::not_set(),
            anheften: ActiveValue::set(false),
        };
        Member::insert(m).exec(&state.conn).await?;
    }
    Ok(StatusCode::OK.into_response())
}

/// 删除群聊
///
/// 只有群主可以删除群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    delete,
    path = "/group/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键")),
    responses(
        (status = 204, description = "成功删除"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn delete_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let g = Group::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find group [{}]", id)))?;
    if g.owner != user.id {
        return Err(AppError::Forbidden(
            "only owner can delete group".to_string(),
        ));
    }
    let res: DeleteResult = Group::delete_by_id(id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::Server(anyhow::anyhow!(
            "cannot delete group [{}]",
            id
        )));
    }
    event!(Level::INFO, "delete group [{}]", id);
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct TransferGroupParams {
    group: Uuid,
    owner: Uuid,
}

/// 转让群主身份
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/group/transfer",
    params(TransferGroupParams),
    responses(
        (status = 200, description = "转让成功"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn transfer_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Query(params): Query<TransferGroupParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let _ = User::find_by_id(params.owner)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find user [{}]",
            params.owner
        )))?;
    let g = Group::find_by_id(params.group)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find group [{}]",
            params.group
        )))?;
    if g.owner != user.id {
        return Err(AppError::Forbidden(
            "only owner can transfer group".to_string(),
        ));
    }
    if user.id == params.owner {
        return Err(AppError::BadRequest("cannot transfer to self".to_string()));
    }
    let mut g = g.into_active_model();
    g.owner = ActiveValue::set(params.owner);
    event!(
        Level::INFO,
        "transfer group [{}] to [{}]",
        params.group,
        params.owner
    );
    Ok(StatusCode::OK.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct PinGroupParams {
    pin: Option<bool>,
}

/// 置顶群聊
///
/// 在请求参数设置 `pin` 为 `true` 时置顶, `false` 时取消置顶
///
/// 默认置顶
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/group/pin/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键"), PinGroupParams),
    responses(
        (status = 200, description = "成功置顶"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn pin_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
    Query(params): Query<PinGroupParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let g = Group::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find group [{}]", id)))?;
    let m = Member::find()
        .filter(member::Column::Group.eq(g.id))
        .filter(member::Column::User.eq(user.id))
        .one(&state.conn)
        .await?;
    let m = m.ok_or(AppError::NotFound(format!("not in group [{}]", id)))?;
    let mut m = m.into_active_model();
    if let Some(pin) = params.pin {
        m.anheften = ActiveValue::set(pin);
    } else {
        m.anheften = ActiveValue::set(true);
    }
    Member::update(m).exec(&state.conn).await?;
    event!(Level::INFO, "pin group [{}]", id);
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// 退出群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    delete,
    path = "/group/exit/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键")),
    responses(
        (status = 204, description = "成功退出"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn exit_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    let g = Group::find_by_id(id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!("cannot find group [{}]", id)))?;
    let m = Member::find()
        .filter(member::Column::Group.eq(g.id))
        .filter(member::Column::User.eq(user.id))
        .one(&state.conn)
        .await?;
    let m = m.ok_or(AppError::NotFound(format!("not in group [{}]", id)))?;
    let res: DeleteResult = Member::delete_by_id(m.id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::Server(anyhow::anyhow!(
            "cannot exit group [{}]",
            id
        )));
    }
    event!(Level::INFO, "exit group [{}]", id);
    Ok(StatusCode::NO_CONTENT.into_response())
}
