use super::view::*;
use crate::*;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(
    paths(
        login::login_handler
    ),
    components(
        schemas(
            error::AppErrorResponse,
            login::LoginRequest, login::LoginResponse
        )
    ),
    tags(
        (name = "user", description = "用户管理")
    )
)]
pub(super) struct ApiDoc;
