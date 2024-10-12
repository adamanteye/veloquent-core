//! JWT 工具库

use axum::{async_trait, extract::FromRequestParts, http::request::Parts, RequestPartsExt};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
pub(super) use jsonwebtoken::{DecodingKey, EncodingKey};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
#[derive(Serialize, Deserialize, ToSchema)]
pub struct JWTPayload {
    /// 用户唯一标识
    pub id: i32,
    /// 过期时间戳
    pub exp: u64,
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
            .map_err(|e| AppError::Unauthorized(format!("Token not found: {}", e)))?;
        let token = jsonwebtoken::decode::<JWTPayload>(
            bearer.token(),
            &JWT_SETTING.get().unwrap().de_key,
            JWT_ALG.get().unwrap(),
        )
        .map_err(|e| AppError::Unauthorized(e.to_string()))?;
        Ok(token.claims)
    }
}
