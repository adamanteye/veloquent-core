use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use tracing::{event, instrument, Level};
use utoipa::ToSchema;

use crate::{
    entity::{prelude::*, *},
    jwt::JWT_SETTING,
};
use crate::{error::AppError, jwt::JWTPayload, AppState};

/// 登录请求体
#[derive(Deserialize, ToSchema, Debug)]
pub struct LoginRequest {
    /// 用户名
    #[schema(example = "yangzheh")]
    name: String,
    /// 密码
    #[schema(example = "123456")]
    passwd: String,
}

impl From<JWTPayload> for String {
    fn from(payload: JWTPayload) -> Self {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &payload,
            &JWT_SETTING.get().unwrap().en_key,
        )
        .unwrap()
    }
}

impl From<i32> for JWTPayload {
    fn from(id: i32) -> Self {
        Self {
            id,
            exp: jsonwebtoken::get_current_timestamp() + JWT_SETTING.get().unwrap().exp,
        }
    }
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
        if validate_passwd(&self.passwd, &user.salt, &user.hash)? {
            event!(Level::INFO, "successfully validate user {:?}", user.name);
            Ok(user.id.into())
        } else {
            event!(Level::INFO, "fail to validate user {:?}", user.name);
            Err(AppError::Unauthorized("wrong password".to_string()))
        }
    }
}

/// 登录响应体
#[derive(ToSchema, Serialize)]
pub struct LoginResponse {
    /// JWT
    #[schema(
        example = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6MSwiZXhwIjoxNzI4ODMzODA2fQ.iZwAs-j5ZRqi26MG7oVf0PL-hEHv3qbf6VnmeCHf5Sc"
    )]
    pub token: String,
}

/// 登录或注册
#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "成功登录", body = LoginResponse),
        (status = 201, description = "成功注册", body = LoginResponse),
        (status = 401, description = "登录失败", body = AppErrorResponse, example = json!({"msg":"wrong password"})),
    ),
    tag = "user"
)]
#[instrument(skip(state))]
pub async fn login_handler(
    State(state): State<AppState>,
    Json(user): Json<LoginRequest>,
) -> Result<Response, AppError> {
    if user.name.is_empty() || user.passwd.is_empty() {
        return Err(AppError::BadRequest("name or passwd is empty".to_string()));
    }
    let payload = match user.validate(&state.conn).await {
        Ok(p) => p,
        Err(AppError::NotFound(_)) => {
            let (hash, salt) = gen_hash_and_salt(&user.passwd)?;
            let new_user = user::ActiveModel {
                name: ActiveValue::Set(user.name.to_string()),
                salt: ActiveValue::Set(salt),
                hash: ActiveValue::Set(hash),
                created_at: ActiveValue::Set(chrono::Utc::now().naive_utc()),
                ..Default::default()
            };
            let res = User::insert(new_user).exec(&state.conn).await?;
            event!(Level::INFO, "create user {:?}", res);
            let res: JWTPayload = res.last_insert_id.into();
            return Ok((
                StatusCode::CREATED,
                Json(LoginResponse { token: res.into() }),
            )
                .into_response());
        }
        Err(e) => return Err(e),
    };
    Ok((
        StatusCode::OK,
        Json(LoginResponse {
            token: payload.into(),
        }),
    )
        .into_response())
}

fn validate_passwd(passwd: &str, salt: &str, hash: &str) -> anyhow::Result<bool> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(passwd);
    h.update(salt);
    let h = h.finalize();
    let mut buf = [0u8; 64];
    let h = base16ct::lower::encode_str(&h, &mut buf)
        .map_err(|e| anyhow::format_err!(e))?
        .to_string();
    if h == hash {
        Ok(true)
    } else {
        Ok(false)
    }
}

fn gen_hash_and_salt(passwd: &str) -> Result<(String, String), anyhow::Error> {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};
    use sha2::{Digest, Sha256};
    let salt: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(30)
        .map(char::from)
        .collect();
    let mut hash = Sha256::new();
    hash.update(passwd);
    hash.update(salt.clone());
    let hash = hash.finalize();
    let mut buf = [0u8; 64];
    let hash = base16ct::lower::encode_str(&hash, &mut buf)
        .map_err(|e| anyhow::format_err!(e))?
        .to_string();
    Ok((hash, salt))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_hash_and_salt() {
        let (hash, salt) = gen_hash_and_salt("123456").unwrap();
        assert_eq!(hash.len(), 64);
        assert_eq!(salt.len(), 30);
        assert!(validate_passwd("123456", &salt, &hash).unwrap());
        assert!(!validate_passwd("1234356", &salt, &hash).unwrap());
    }
}
