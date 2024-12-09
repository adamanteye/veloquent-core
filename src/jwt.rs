//! JWT 工具库

use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
pub(super) use jsonwebtoken::{DecodingKey, EncodingKey};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
#[cfg(feature = "dev")]
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::AppError;

pub(super) static JWT_ALG: OnceCell<jsonwebtoken::Validation> = OnceCell::new();
pub(super) static JWT_SETTING: OnceCell<JwtSetting> = OnceCell::new();

#[doc(hidden)]
pub(super) struct JwtSetting {
    pub(super) exp: u64,
    pub(super) de_key: DecodingKey,
    pub(super) en_key: EncodingKey,
}

/// JWT 载荷
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize, Deserialize, Debug)]
pub struct JWTPayload {
    /// 用户唯一标识
    pub id: Uuid,
    /// 过期时间戳
    pub exp: u64,
}

impl JWTPayload {
    pub(super) async fn to_user(
        &self,
        conn: &sea_orm::DatabaseConnection,
    ) -> Result<super::entity::user::Model, AppError> {
        use sea_orm::EntityTrait;
        super::entity::prelude::User::find_by_id(self.id)
            .one(conn)
            .await?
            .ok_or(AppError::NotFound(format!(
                "cannot find user [{}]",
                self.id
            )))
    }
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

impl From<Uuid> for JWTPayload {
    fn from(id: Uuid) -> Self {
        Self {
            id,
            exp: jsonwebtoken::get_current_timestamp() + JWT_SETTING.get().unwrap().exp,
        }
    }
}

impl TryFrom<&str> for JWTPayload {
    type Error = AppError;
    fn try_from(token: &str) -> Result<Self, Self::Error> {
        jsonwebtoken::decode::<JWTPayload>(
            token,
            &JWT_SETTING.get().unwrap().de_key,
            JWT_ALG.get().unwrap(),
        )
        .map_err(|e| AppError::Unauthorized(format!("invalid JWT: [{}]", e)))
        .map(|t| t.claims)
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for JWTPayload
where
    S: Send + Sync,
{
    type Rejection = AppError;
    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| AppError::BadRequest(format!("token not found: [{}]", e)))?;
        let token = bearer.token().try_into()?;
        Ok(token)
    }
}
