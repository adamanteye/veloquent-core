use super::*;
use entity::{prelude::User, user};

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize, Debug)]
pub struct UserList {
    /// 用户主键列表
    pub users: Vec<Uuid>,
}

/// 各条件之间用与连接
///
/// 没有提供字段的条件不参与查询
#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub struct UserFindRequest {
    /// 用户名
    pub name: Option<String>,
    /// 别名
    pub alias: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 电话
    pub phone: Option<String>,
}

impl UserList {
    pub async fn find(
        params: UserFindRequest,
        conn: &DatabaseConnection,
    ) -> Result<Self, AppError> {
        let users = User::find()
            .filter(
                Condition::all()
                    .add(user::Column::Name.like(format!("%{}%", params.name.unwrap_or_default())))
                    .add(
                        user::Column::Alias.like(format!("%{}%", params.alias.unwrap_or_default())),
                    )
                    .add(
                        user::Column::Email.like(format!("%{}%", params.email.unwrap_or_default())),
                    )
                    .add(
                        user::Column::Phone.like(format!("%{}%", params.phone.unwrap_or_default())),
                    ),
            )
            .all(conn)
            .await?;

        Ok(Self {
            users: users.into_iter().map(|u| u.id).collect(),
        })
    }
}
/// 查找用户
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/user",
    params(UserFindRequest),
    responses(
        (status = 200, description = "获取成功", body = UserList),
    ),
    tag = "user"
))]
#[instrument(skip(state, _payload))]
pub async fn find_user_handler(
    State(state): State<AppState>,
    _payload: JWTPayload,
    Query(params): Query<UserFindRequest>,
) -> Result<Json<UserList>, AppError> {
    let users = UserList::find(params, &state.conn).await?;
    event!(Level::DEBUG, "conditional find users: [{:?}]", users);
    Ok(Json(users))
}
