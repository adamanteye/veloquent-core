use super::*;
use entity::{prelude::User, user};

#[derive(prost::Message, ToSchema)]
pub struct UserList {
    /// 用户主键列表
    #[prost(message, repeated, tag = "1")]
    pub users: Vec<String>,
}

/// 各条件之间用与连接
#[derive(IntoParams, Deserialize, Debug)]
pub struct UserFindParams {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
}

impl UserList {
    pub async fn find(params: UserFindParams, conn: &DatabaseConnection) -> Result<Self, AppError> {
        // let users = Query::select().column()
        let users = User::find()
            .apply_if(params.name, |q, v| q.filter(user::Column::Name.like(v)))
            .apply_if(params.alias, |q, v| q.filter(user::Column::Alias.like(v)))
            .apply_if(params.email, |q, v| q.filter(user::Column::Email.like(v)))
            .apply_if(params.phone, |q, v| q.filter(user::Column::Phone.like(v)))
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
    params(UserFindParams),
    responses(
        (status = 200, description = "获取成功, 返回 protobuf 数据", body = UserList),
    ),
    tag = "user"
)]
#[instrument(skip(state, _payload))]
pub async fn find_user_handler(
    State(state): State<AppState>,
    _payload: JWTPayload,
    Query(params): Query<UserFindParams>,
) -> Result<Protobuf<UserList>, AppError> {
    let users = UserList::find(params, &state.conn).await?;
    event!(Level::DEBUG, "conditional find users: [{:?}]", users);
    Ok(Protobuf(users))
}
