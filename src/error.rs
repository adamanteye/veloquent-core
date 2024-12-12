//! 错误响应

use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
#[cfg(feature = "dev")]
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
#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct AppErrorResponse {
    /// 错误信息
    msg: String,
    /// 当前 API 版本
    ver: &'static str,
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
        (
            status,
            Json(AppErrorResponse {
                msg: message,
                ver: env!("CARGO_PKG_VERSION"),
            }),
        )
            .into_response()
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn convert_std_error_to_response() {
        let res: axum::response::Response =
            AppError::from(std::io::Error::new(std::io::ErrorKind::Other, "test")).into_response();

        assert_eq!(res.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }
}
