use super::*;
use entity::{prelude::User, user};
use login::LoginResponse;
use utility::{gen_hash_and_salt, good_email, good_phone};

/// 用户创建请求体
///
/// 不提供该字段表示不进行设置或修改, 提供空字符串表示置为默认
#[derive(Deserialize, Debug)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct RegisterProfile {
    /// 用户名
    pub name: String,
    /// 别名
    pub alias: Option<String>,
    /// 电话号码
    pub phone: String,
    /// 性别
    ///
    /// | 数值 | 说明 |
    /// | --- | --- |
    /// | 0 | 未指定 |
    /// | 1 | 女性 |
    /// | 2 | 男性 |
    pub gender: Option<i32>,
    /// 个性简介
    pub bio: Option<String>,
    /// 个人链接
    pub link: Option<String>,
    /// 密码
    pub password: String,
    /// 邮件地址
    pub email: String,
}

impl TryFrom<RegisterProfile> for user::ActiveModel {
    type Error = AppError;
    fn try_from(p: RegisterProfile) -> Result<Self, Self::Error> {
        if !p.name.is_empty() && !p.password.is_empty() {
            if p.gender.is_some_and(|g| g.is_negative()) {
                Err(AppError::BadRequest("gender not valid".to_string()))
            } else if !p.email.is_empty() && !good_email(&p.email) {
                Err(AppError::BadRequest("invalid email".to_string()))
            } else if !p.email.is_empty() && !good_phone(&p.phone) {
                Err(AppError::BadRequest("invalid phone".to_string()))
            } else {
                let (hash, salt) = gen_hash_and_salt(&p.password)?;
                Ok(user::ActiveModel {
                    id: ActiveValue::not_set(),
                    name: ActiveValue::Set(p.name),
                    alias: ActiveValue::Set(p.alias),
                    phone: ActiveValue::Set(p.phone),
                    hash: ActiveValue::set(hash),
                    salt: ActiveValue::set(salt),
                    created_at: ActiveValue::not_set(),
                    gender: ActiveValue::set(p.gender.unwrap_or_default()),
                    email: ActiveValue::Set(p.email),
                    bio: ActiveValue::Set(p.bio),
                    avatar: ActiveValue::not_set(),
                    link: ActiveValue::Set(p.link),
                })
            }
        } else {
            Err(AppError::BadRequest(
                "name and password must be provided".to_string(),
            ))
        }
    }
}

/// 注册新用户
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/register",
    request_body = RegisterProfile,
    responses(
        (status = 201, description = "注册成功", body = LoginResponse),
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn register_handler(
    State(state): State<AppState>,
    Json(profile): Json<RegisterProfile>,
) -> Result<Response, AppError> {
    let user = user::ActiveModel::try_from(profile)?;
    let res = User::insert(user).exec(&state.conn).await?;
    event!(Level::INFO, "create user {:?}", res);
    let res: JWTPayload = res.last_insert_id.into();
    Ok((
        StatusCode::CREATED,
        Json(LoginResponse { token: res.into() }),
    )
        .into_response())
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct UserProfile {
    /// 主键
    pub id: Uuid,
    /// 用户名
    pub name: String,
    /// 别名
    pub alias: String,
    /// 邮箱
    pub email: String,
    /// 电话
    pub phone: String,
    /// 个人链接
    pub link: String,
    /// 性别
    pub gender: i32,
    /// 个性简介
    pub bio: String,
    /// 头像
    pub avatar: Uuid,
    /// 创建时间
    ///
    /// UTC 毫秒时间戳
    pub created_at: i64,
}

impl From<user::Model> for UserProfile {
    fn from(user: user::Model) -> Self {
        Self {
            id: user.id.into(),
            name: user.name,
            gender: user.gender,
            alias: user.alias.unwrap_or_default(),
            email: user.email,
            phone: user.phone,
            created_at: user.created_at.and_utc().timestamp_millis(),
            avatar: user.avatar.unwrap_or_default(),
            bio: user.bio.unwrap_or_default(),
            link: user.link.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn convert_profile_from_entity() {
        let created_at = chrono::Utc::now().naive_utc();
        let user = user::Model {
            id: Uuid::from_str("264107cf-8559-41b0-a8fe-074531695bf6").unwrap(),
            name: "test".to_string(),
            alias: Option::None,
            email: "adamanteye@example.com".to_string(),
            phone: "1234567890".to_string(),
            created_at,
            avatar: Option::None,
            bio: Option::None,
            link: Option::None,
            gender: 0,
            salt: "KxlaYxELSZSGYCEsm5dE00BTTxnZ10".to_string(),
            hash: "74491363c6cc8c851ed7e1ea3279741795cf4e1f9534b125562ff7030f295eb7".to_string(),
        };
        assert_eq!(
            UserProfile::from(user),
            UserProfile {
                id: Uuid::from_str("264107cf-8559-41b0-a8fe-074531695bf6").unwrap(),
                name: "test".to_string(),
                email: "adamanteye@example.com".to_string(),
                phone: "1234567890".to_string(),
                created_at: created_at.and_utc().timestamp_millis(),
                gender: 0,
                avatar: uuid::Uuid::default(),
                alias: String::default(),
                bio: String::default(),
                link: String::default(),
            }
        );
    }
}

#[cfg_attr(feature = "dev", derive(IntoParams))]
#[derive(Deserialize, Debug)]
pub struct UserProfileParams {
    /// 用户唯一主键
    pub id: Option<Uuid>,
}

/// 获取用户信息
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/user/profile",
    params(UserProfileParams),
    responses(
        (status = 200, description = "获取成功", body = UserProfile),
        (status = 400, description = "提取 Authorization Bearer 失败", body = AppErrorResponse, example = json!({"msg":"token not found: [invalid HTTP header (authorization)]","ver": "0.1.1"})),
        (status = 401, description = "验证用户失败", body = AppErrorResponse, example = json!({"msg":"invalid JWT: [InvalidSignature]","ver": "0.1.1"}))
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn get_profile_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Query(params): Query<UserProfileParams>,
) -> Result<Json<UserProfile>, AppError> {
    let user: Option<user::Model> = User::find_by_id(params.id.unwrap_or(payload.id))
        .one(&state.conn)
        .await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    event!(Level::DEBUG, "get user profile: [{:?}]", user);
    Ok(Json(UserProfile::from(user)))
}

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Clone, Debug, Deserialize)]
pub struct UserProfileEdition {
    /// 用户名
    pub name: Option<String>,
    /// 别名
    pub alias: Option<String>,
    /// 邮箱
    pub email: Option<String>,
    /// 电话
    pub phone: Option<String>,
    /// 个人链接
    pub link: Option<String>,
    /// 性别
    pub gender: Option<i32>,
    /// 个性简介
    pub bio: Option<String>,
}

impl TryFrom<UserProfileEdition> for user::ActiveModel {
    type Error = AppError;

    fn try_from(value: UserProfileEdition) -> Result<Self, Self::Error> {
        Ok(user::ActiveModel {
            id: ActiveValue::not_set(),
            name: match value.name {
                Some(n) => ActiveValue::set(n),
                None => ActiveValue::not_set(),
            },
            salt: ActiveValue::not_set(),
            hash: ActiveValue::not_set(),
            avatar: ActiveValue::not_set(),
            created_at: ActiveValue::not_set(),
            gender: match value.gender {
                Some(g) => ActiveValue::set(g),
                None => ActiveValue::not_set(),
            },
            bio: ActiveValue::set(value.bio),
            alias: ActiveValue::set(value.alias),
            email: match value.email {
                Some(e) => {
                    if !good_email(&e) {
                        return Err(AppError::BadRequest("invalid email".to_string()));
                    }
                    ActiveValue::set(e)
                }
                None => ActiveValue::not_set(),
            },
            phone: match value.phone {
                Some(p) => {
                    if !good_phone(&p) {
                        return Err(AppError::BadRequest("invalid phone".to_string()));
                    }
                    ActiveValue::set(p)
                }
                None => ActiveValue::not_set(),
            },
            link: ActiveValue::set(value.link),
        })
    }
}

/// 修改用户信息
#[cfg_attr(feature = "dev",
utoipa::path(
    put,
    path = "/user/profile",
    request_body = UserProfileEdition,
    responses(
        (status = 200, description = "更新成功"),
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn update_profile_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Json(edition): Json<UserProfileEdition>,
) -> Result<Response, AppError> {
    User::find_by_id(payload.id)
        .one(&state.conn)
        .await?
        .ok_or(AppError::NotFound(format!(
            "cannot find user: [{}]",
            payload.id
        )))?;
    let mut user = user::ActiveModel::try_from(edition)?;
    user.id = ActiveValue::set(payload.id);
    User::update(user).exec(&state.conn).await?;
    event!(Level::INFO, "update user [{}]", payload.id);
    Ok(StatusCode::OK.into_response())
}

/// 删除用户自己
#[cfg_attr(feature = "dev", utoipa::path(delete, path = "/user/profile", responses((status = 204, description = "删除成功")), tag = "user"))]
#[instrument(skip(state))]
pub async fn delete_user_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Response, AppError> {
    let res: DeleteResult = User::delete_by_id(payload.id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "cannot delete user [{}]",
            payload.id
        )));
    }
    event!(Level::INFO, "delete user [{}]", payload.id);
    Ok(StatusCode::NO_CONTENT.into_response())
}

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
