use axum::{extract::State, Json};
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    entity::{prelude::*, *},
    jwt::JWT_SETTING,
};
use crate::{error::AppError, jwt::JWTPayload, AppState};

/// 登录请求体
#[derive(Deserialize, ToSchema)]
pub struct LoginRequest {
    /// 用户名
    #[schema(example = "yangzheh")]
    name: String,
    /// 密码
    #[schema(example = "123456")]
    passwd: String,
}

impl LoginRequest {
    pub async fn validate(&self, conn: &DatabaseConnection) -> Result<JWTPayload, AppError> {
        use sha2::{Digest, Sha256};
        let user: Option<user::Model> = User::find()
            .filter(user::Column::Name.eq(&self.name))
            .one(conn)
            .await?;
        let user = user.ok_or(AppError::NotFound(format!(
            "user not exist: [{}]",
            &self.name
        )))?;
        let mut hash = Sha256::new();
        hash.update(self.passwd.clone());
        hash.update(user.salt);
        let hash = hash.finalize();
        let mut buf = [0u8; 64];
        let hash = base16ct::lower::encode_str(&hash, &mut buf)
            .map_err(|e| anyhow::format_err!(e))?
            .to_string();
        if hash == user.hash {
            Ok(JWTPayload {
                id: user.id,
                exp: jsonwebtoken::get_current_timestamp()
                    + crate::jwt::JWT_SETTING.get().unwrap().exp,
            })
        } else {
            Err(AppError::Unauthorized(format!(
                "password is wrong [{}]",
                &self.name
            )))
        }
    }
}

/// 登录响应体
#[derive(ToSchema, Serialize)]
pub struct LoginResponse {
    /// JWT
    pub token: String,
    /// 是否注册
    pub is_created: bool,
}

/// 登录或注册
#[utoipa::path(
    post,
    path = "/login",
    request_body = LoginPost,
    responses(
        (status = 200, description = "成功登录", body = JWTPayload),
        (status = 401, description = "登录失败", body = AppErrorResponse),
    ),
    tag = "User"
)]
pub async fn login_handler(
    State(state): State<AppState>,
    Json(user): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
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
            let _ = User::insert(new_user).exec(&state.conn).await?;
            user.validate(&state.conn).await?
        }
        Err(e) => return Err(e),
    };
    let payload = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &payload,
        &JWT_SETTING.get().unwrap().en_key,
    )?;
    Ok(Json(LoginResponse {
        token: payload,
        is_created: false,
    }))
}

fn gen_hash_and_salt(passwd: &String) -> Result<(String, String), anyhow::Error> {
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
