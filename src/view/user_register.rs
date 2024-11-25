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
