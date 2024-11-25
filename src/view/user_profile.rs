use super::*;
use entity::{prelude::User, user};
use utility::{good_email, good_phone};

#[allow(clippy::derive_partial_eq_without_eq)]
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Clone, PartialEq, Serialize, Debug)]
pub struct UserProfile {
    /// 主键
    pub id: String,
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
    pub avatar: String,
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
            avatar: user.avatar.unwrap_or_default().to_string(),
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
                id: "264107cf-8559-41b0-a8fe-074531695bf6".to_string(),
                name: "test".to_string(),
                email: "adamanteye@example.com".to_string(),
                phone: "1234567890".to_string(),
                created_at: created_at.and_utc().timestamp_millis(),
                gender: 0,
                avatar: uuid::Uuid::default().to_string(),
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
