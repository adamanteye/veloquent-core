use super::*;
use entity::{prelude::User, user};

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, prost::Message, ToSchema)]
pub struct UserProfile {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub name: String,
    #[prost(string, tag = "3")]
    pub alias: String,
    #[prost(string, tag = "4")]
    pub email: String,
    #[prost(string, tag = "5")]
    pub phone: String,
    #[prost(string, tag = "6")]
    pub link: String,
    #[prost(int32, tag = "7")]
    pub gender: i32,
    #[prost(string, tag = "8")]
    pub bio: String,
    #[prost(string, tag = "9")]
    pub avatar: String,
    #[prost(string, tag = "10")]
    pub created_at: String,
}

impl From<user::Model> for UserProfile {
    fn from(user: user::Model) -> Self {
        Self {
            id: user.id.into(),
            name: user.name,
            gender: user.gender,
            alias: user.alias.unwrap_or_default(),
            email: user.email.unwrap_or_default(),
            phone: user.phone.unwrap_or_default(),
            created_at: user.created_at.to_string(),
            avatar: user.avatar.unwrap_or_default().to_string(),
            bio: user.bio.unwrap_or_default(),
            link: user.link.unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use coverage_helper::test;
    use std::str::FromStr;

    #[test]
    fn test_profile_from_entity() {
        let created_at = chrono::Utc::now().naive_utc();
        let user = user::Model {
            id: Uuid::from_str("264107cf-8559-41b0-a8fe-074531695bf6").unwrap(),
            name: "test".to_string(),
            alias: Option::None,
            email: Option::None,
            phone: Option::None,
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
                email: String::default(),
                phone: String::default(),
                created_at: created_at.to_string(),
                gender: 0,
                avatar: uuid::Uuid::default().to_string(),
                alias: String::default(),
                bio: String::default(),
                link: String::default(),
            }
        );
    }
}

/// 获取用户信息
///
/// 返回的格式为 protobuf 数据
#[utoipa::path(
    get,
    path = "/user/profile",
    responses(
        (status = 200, description = "获取成功, 返回 protobuf 数据", body = UserProfile),
        (status = 400, description = "提取 Authorization Bearer 失败", body = AppErrorResponse, example = json!({"msg":"token not found: [invalid HTTP header (authorization)]","ver": "0.1.1"})),
        (status = 401, description = "验证用户失败", body = AppErrorResponse, example = json!({"msg":"invalid JWT: [InvalidSignature]","ver": "0.1.1"}))
    ),
    tag = "user"
)]
pub async fn get_self_profile_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Protobuf<UserProfile>, AppError> {
    let user: Option<user::Model> = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user [{}]",
        payload.id
    )))?;
    Ok(Protobuf(UserProfile::from(user)))
}
