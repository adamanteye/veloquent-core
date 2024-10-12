//! 错误响应

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use utoipa::ToSchema;

/// 查看 [HTTP status code wiki](https://en.wikipedia.org/wiki/List_of_HTTP_status_codes)
#[derive(Debug)]
pub enum AppError {
    /// 400 Bad Request
    BadRequest(String),
    /// 401 Unauthorized
    Unauthorized(String),
    /// 403 Forbidden
    Forbidden(String),
    /// 404 Not Found
    NotFound(String),
    /// 409 Conflict
    Conflict(String),
    /// 500 Internal Server Error
    Server(anyhow::Error),
}

/// 错误响应
#[doc(hidden)]
#[derive(Serialize, ToSchema)]
pub struct AppErrorResponse {
    /// 错误信息
    msg: String,
}

impl<E> From<E> for AppError
where
    E: Into<anyhow::Error>,
{
    fn from(value: E) -> Self {
        Self::Server(value.into())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            Self::Server(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, msg),
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
        };
        (status, Json(AppErrorResponse { msg: message })).into_response()
    }
}
