use super::*;
use entity::{prelude::User, user};
use utility::validate_passwd;

/// 登录请求体
#[derive(Deserialize, Debug)]
#[cfg_attr(test, derive(Serialize))]
#[cfg_attr(feature = "dev", derive(ToSchema))]
pub struct LoginRequest {
    /// 用户名
    #[cfg_attr(feature = "dev", schema(example = "yangzheh"))]
    #[cfg(test)]
    pub name: String,
    #[cfg(not(test))]
    name: String,
    /// 密码
    #[cfg_attr(feature = "dev", schema(example = "123456"))]
    #[cfg(test)]
    pub password: String,
    #[cfg(not(test))]
    password: String,
}

impl LoginRequest {
    async fn validate(&self, conn: &DatabaseConnection) -> Result<JWTPayload, AppError> {
        let user: Option<user::Model> = User::find()
            .filter(user::Column::Name.eq(&self.name))
            .one(conn)
            .await?;
        let user = user.ok_or(AppError::NotFound(format!(
            "user not exist: [{}]",
            &self.name
        )))?;
        if validate_passwd(&self.password, &user.salt, &user.hash)? {
            event!(Level::INFO, "successfully validate user {:?}", user.name);
            Ok(user.id.into())
        } else {
            event!(Level::INFO, "fail to validate user {:?}", user.name);
            Err(AppError::Unauthorized("wrong password".to_string()))
        }
    }
}

/// 登录响应体
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[cfg_attr(test, derive(Deserialize))]
#[derive(Serialize)]
pub struct LoginResponse {
    /// JWT token
    #[cfg_attr(
        feature = "dev",
        schema(
            example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6IjQ1OWE0MjBhLTQxNDMtNGFkYy1hZjgxLWQ1NGRmMjg4YmJlZCIsImV4cCI6MTcyOTE1MzUyNn0.lAM0QjzaJvaB8KgTcnRfUrEDOBwiBLIJ2u6yOivzsSk"
        )
    )]
    pub token: String,
}

/// 登录
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "登录成功", body = LoginResponse),
        (status = 401, description = "登录失败", body = AppErrorResponse, example = json!({"msg":"wrong password","ver": "0.1.1"})),
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn login_handler(
    State(state): State<AppState>,
    Json(user): Json<LoginRequest>,
) -> Result<Response, AppError> {
    if user.name.is_empty() || user.password.is_empty() {
        return Err(AppError::BadRequest("name or passwd is empty".to_string()));
    }
    let payload = user.validate(&state.conn).await?;
    event!(Level::INFO, "user login [{}]", user.name);
    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            token: payload.into(),
        }),
    )
        .into_response())
}

/// 刷新 JWT
#[cfg_attr(feature = "dev",
utoipa::path(
    get,
    path = "/renew",
    responses(
        (status = 200, description = "刷新成功", body = LoginResponse),
    ),
    tag = "user"
))]
#[instrument(skip(state))]
pub async fn renew_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Json<LoginResponse>, AppError> {
    user::Model::from_uuid(payload.id, &state.conn).await?;
    let token = payload.id.into();
    Ok(Json(LoginResponse { token }))
}
