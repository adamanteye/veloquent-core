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
    Path(id): Path<Uuid>,
) -> Result<Json<GroupProfile>, AppError> {
    let g = GroupProfile::from_group_id((id, &state.conn)).await?;
    Ok(Json(g))
}

impl GroupProfile {
    async fn from_group_id((id, conn): (Uuid, &DatabaseConnection)) -> Result<Self, AppError> {
        let g = Group::find_by_id(id)
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!("cannot find group [{id}]")))?;
        let members = Member::find()
            .filter(member::Column::Group.eq(g.id))
            .filter(member::Column::Permission.eq(0))
            .all(conn)
            .await?;
        let members: Vec<Uuid> = members.into_iter().map(|m| m.user).collect();
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
        })
    }
}

/// 创建群聊
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/group/new",
    request_body = GroupPost,
    responses(
        (status = 200, description = "获取成功", body = GroupProfile),
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
        "cannot find user {}",
        payload.id
    )))?;
    let mut members = req.members;
    members.push(user.id);
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
        };
        Member::insert(m).exec(&state.conn).await?;
    }
    let g = GroupProfile::from_group_id((g, &state.conn)).await?;
    Ok(Json(g))
}