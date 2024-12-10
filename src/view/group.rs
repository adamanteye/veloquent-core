use super::*;
use entity::{
    group, member,
    prelude::{Group, Member, Session, User},
    session,
};

impl group::Model {
    async fn from_uuid(id: Uuid, conn: &DatabaseConnection) -> Result<Self, AppError> {
        Group::find_by_id(id)
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!("cannot find group [{id}]")))
    }
}

impl member::Model {
    async fn from_group_and_user(
        group: Uuid,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        Member::find()
            .filter(member::Column::Group.eq(group))
            .filter(member::Column::User.eq(user))
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!(
                "user [{user}] not in group [{group}]"
            )))
    }
}

impl From<(Uuid, Uuid)> for member::ActiveModel {
    fn from(value: (Uuid, Uuid)) -> Self {
        member::ActiveModel {
            id: ActiveValue::not_set(),
            group: ActiveValue::set(value.0),
            user: ActiveValue::set(value.1),
            permission: ActiveValue::set(0),
            created_at: ActiveValue::not_set(),
            anheften: ActiveValue::set(false),
        }
    }
}

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
        let g = group::Model::from_uuid(id, conn).await?;
        let members = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::Permission.ne(-1))
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
    let user = payload.to_user(&state.conn).await?;
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
    let user = payload.to_user(&state.conn).await?;
    let mut members = req.members;
    members.push(user.id);
    members.sort();
    members.dedup();
    if members.len() < 2 {
        return Err(AppError::BadRequest("at least 2 members".to_string()));
    }
    let s = session::ActiveModel::default();
    let s = Session::insert(s).exec(&state.conn).await?.last_insert_id;
    let n = session::ActiveModel::default();
    let n = Session::insert(n).exec(&state.conn).await?.last_insert_id;
    let g = group::ActiveModel {
        id: ActiveValue::not_set(),
        name: ActiveValue::set(req.name),
        owner: ActiveValue::set(user.id),
        session: ActiveValue::set(s),
        notice: ActiveValue::set(n),
        created_at: ActiveValue::not_set(),
    };
    let g = Group::insert(g).exec(&state.conn).await?.last_insert_id;
    for m in members {
        let m = member::ActiveModel::from((g, m));
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
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(id, &state.conn).await?;
    member::Model::from_group_and_user(g.id, user.id, &state.conn).await?;
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
        let mut m = member::ActiveModel::from((g.id, u));
        m.permission = ActiveValue::set(-1);
        Member::insert(m).exec(&state.conn).await?;
    }
    Ok(StatusCode::OK.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct GroupDeleteParams {
    /// 要移除的用户
    user: Option<Uuid>,
}

/// 删除群聊或移除群聊中的成员
///
/// 只有群主可以删除群聊
///
/// 管理员和群主可以移除群聊中的成员
#[cfg_attr(feature = "dev",
utoipa::path(
    delete,
    path = "/group/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键"), GroupDeleteParams),
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
    Query(params): Query<GroupDeleteParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(id, &state.conn).await?;
    if let Some(u) = params.user {
        let is_admin = member::Model::from_group_and_user(g.id, user.id, &state.conn)
            .await?
            .permission
            == 1;
        if !is_admin && g.owner != user.id {
            return Err(AppError::Forbidden(
                "only admin or owner can delete member".to_string(),
            ));
        }
        let m = member::Model::from_group_and_user(g.id, u, &state.conn).await?;
        let res: DeleteResult = Member::delete_by_id(m.id).exec(&state.conn).await?;
        if res.rows_affected == 0 {
            return Err(AppError::Server(anyhow::anyhow!(
                "cannot delete member [{u}]"
            )));
        }
        event!(Level::INFO, "delete member [{u}] from group [{id}]");
    } else {
        if g.owner != user.id {
            return Err(AppError::Forbidden(
                "only owner can delete group".to_string(),
            ));
        }
        let res: DeleteResult = Group::delete_by_id(id).exec(&state.conn).await?;
        if res.rows_affected == 0 {
            return Err(AppError::Server(anyhow::anyhow!(
                "cannot delete group [{id}]",
            )));
        }
        event!(Level::INFO, "delete group [{id}]",);
    }
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct ApproveMemberParams {
    /// 群成员
    member: Option<Uuid>,
    /// 是否拒绝
    deny: Option<bool>,
}

/// 群成员审批
///
/// 允许或拒绝加入群聊的请求
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/group/approve/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键"), ApproveMemberParams),
    responses(
        (status = 200, description = "修改成功"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn approve_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(group): Path<Uuid>,
    Query(params): Query<ApproveMemberParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(group, &state.conn).await?;
    let is_admin = Member::is_admin(group, user.id, &state.conn).await?;
    if !is_admin && g.owner != user.id {
        return Err(AppError::Forbidden(
            "only admin or owner can approve member".to_string(),
        ));
    }
    let deny = params.deny.unwrap_or(false);
    if let Some(member) = params.member {
        let m = member::Model::from_group_and_user(g.id, member, &state.conn).await?;
        return if m.permission != -1 {
            Err(AppError::BadRequest(format!(
                "member [{member}] already in group [{group}]"
            )))
        } else if deny {
            let res: DeleteResult = Member::delete_by_id(m.id).exec(&state.conn).await?;
            if res.rows_affected == 0 {
                Err(AppError::Server(anyhow::anyhow!(
                    "cannot delete member [{member}]"
                )))
            } else {
                Ok(StatusCode::NO_CONTENT.into_response())
            }
        } else {
            let mut m = m.into_active_model();
            m.permission = ActiveValue::set(0);
            Member::update(m).exec(&state.conn).await?;
            Ok(StatusCode::OK.into_response())
        };
    }
    Ok(StatusCode::OK.into_response())
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub(super) struct ManageGroupParams {
    /// 新群主
    owner: Option<Uuid>,
    /// 管理员
    admin: Option<Uuid>,
    /// 是否移除管理员或群成员
    ///
    /// 字段为空的时候默认 `false`
    remove: Option<bool>,
    /// 群成员
    member: Option<Uuid>,
}

impl Member {
    async fn is_admin(
        group: Uuid,
        user: Uuid,
        conn: &DatabaseConnection,
    ) -> Result<bool, AppError> {
        let m = Member::find()
            .filter(member::Column::Group.eq(group))
            .filter(member::Column::User.eq(user))
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!(
                "member [{user}] not in [{group}]",
            )))?;
        Ok(m.permission == 1)
    }
}

/// 群聊管理
///
/// 转移群主, 添加或移除管理员, 添加或移除群成员
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/group/manage/{id}",
    params(("id" = Uuid, Path, description = "群聊的唯一主键"), ManageGroupParams),
    responses(
        (status = 200, description = "修改成功"),
    ),
    tag = "group"
))]
#[instrument(skip(state))]
pub async fn manage_group_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Path(group): Path<Uuid>,
    Query(params): Query<ManageGroupParams>,
) -> Result<impl IntoResponse, AppError> {
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(group, &state.conn).await?;
    if let Some(owner) = params.owner {
        if g.owner != user.id {
            return Err(AppError::Forbidden(
                "only owner can transfer group".to_string(),
            ));
        }
        if user.id == owner {
            return Err(AppError::BadRequest("cannot transfer to self".to_string()));
        }
        let mut g = g.clone().into_active_model();
        g.owner = ActiveValue::set(owner);
        Group::update(g).exec(&state.conn).await?;
        event!(Level::INFO, "transfer group [{group}] to [{owner}]");
    }
    let remove = params.remove.unwrap_or(false);
    if let Some(admin) = params.admin {
        if g.owner != user.id {
            return Err(AppError::Forbidden("only owner can edit admin".to_string()));
        }
        let m = member::Model::from_group_and_user(g.id, admin, &state.conn).await?;
        let mut m = m.into_active_model();
        m.permission = ActiveValue::set(if remove { 0 } else { 1 });
        Member::update(m).exec(&state.conn).await?;
        event!(
            Level::INFO,
            "{} admin [{admin}] {} group [{group}]",
            if remove { "remove" } else { "add" },
            if remove { "off" } else { "into" },
        );
    }
    if let Some(member) = params.member {
        let is_admin = Member::is_admin(group, user.id, &state.conn).await?;
        if !is_admin && g.owner != user.id {
            return Err(AppError::Forbidden(
                "only admin or owner can edit member".to_string(),
            ));
        }
        let m = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::User.eq(member))
            .one(&state.conn)
            .await?;
        return match m {
            Some(m) => {
                if m.permission == 1 && is_admin && remove {
                    return Err(AppError::Forbidden(
                        "only owner can remove admin".to_string(),
                    ));
                }
                if remove {
                    if member == g.owner {
                        return Err(AppError::BadRequest("cannot remove owner".to_string()));
                    } else if member == user.id {
                        return Err(AppError::BadRequest("cannot remove self".to_string()));
                    }
                    Member::delete_by_id(m.id).exec(&state.conn).await?;
                } else {
                    return Err(AppError::BadRequest(format!(
                        "[{member}] already in group [{}]",
                        g.id
                    )));
                }
                event!(Level::INFO, "remove member [{member}] from group [{group}]");
                Ok(StatusCode::NO_CONTENT.into_response())
            }
            None => {
                let m = member::ActiveModel::from((g.id, member));
                Member::insert(m).exec(&state.conn).await?;
                event!(Level::INFO, "add member [{member}] into group [{group}]");
                Ok(StatusCode::OK.into_response())
            }
        };
    }
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
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(id, &state.conn).await?;
    let m = member::Model::from_group_and_user(g.id, user.id, &state.conn).await?;
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
    let user = payload.to_user(&state.conn).await?;
    let g = group::Model::from_uuid(id, &state.conn).await?;
    let m = member::Model::from_group_and_user(g.id, user.id, &state.conn).await?;
    if g.owner == user.id {
        return Err(AppError::Forbidden(
            "owner cannot exit group before transferring the group".to_string(),
        ));
    }
    let res: DeleteResult = Member::delete_by_id(m.id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::Server(anyhow::anyhow!(
            "cannot exit group [{id}]",
        )));
    }
    event!(Level::INFO, "user [{}] exit group [{id}]", user.id);
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn default_active_value_is_not_set() {
        let v = session::ActiveModel::default();
        assert_eq!(
            v,
            session::ActiveModel {
                id: ActiveValue::not_set(),
                created_at: ActiveValue::not_set(),
            }
        );
    }
}
