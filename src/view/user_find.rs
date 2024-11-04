use super::*;
use entity::{prelude::User, user};

#[derive(prost::Message, ToSchema)]
pub struct UserList {
    /// 用户主键列表
    #[prost(message, repeated, tag = "1")]
    pub users: Vec<String>,
}

/// 各条件之间用或连接
///
/// 没有提供字段的条件不参与查询
#[derive(IntoParams, Deserialize, Debug)]
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
                Condition::any()
                    .add(user::Column::Name.like(params.name.unwrap_or_default().to_string()))
                    .add(user::Column::Alias.like(params.alias.unwrap_or_default().to_string()))
                    .add(user::Column::Email.like(params.email.unwrap_or_default().to_string()))
                    .add(user::Column::Phone.like(params.phone.unwrap_or_default().to_string())),
            )
            .all(conn)
            .await?;

        Ok(Self {
            users: users.into_iter().map(|u| u.id.to_string()).collect(),
        })
    }
}
/// 查找用户
///
/// 返回的格式为 protobuf 数据
#[utoipa::path(
    get,
    path = "/user",
    params(UserFindRequest),
    responses(
        (status = 200, description = "获取成功, 返回 protobuf 数据", body = UserList),
    ),
    tag = "user"
)]
#[instrument(skip(state, _payload))]
pub async fn find_user_handler(
    State(state): State<AppState>,
    _payload: JWTPayload,
    Query(params): Query<UserFindRequest>,
) -> Result<Protobuf<UserList>, AppError> {
    let users = UserList::find(params, &state.conn).await?;
    event!(Level::DEBUG, "conditional find users: [{:?}]", users);
    Ok(Protobuf(users))
}
