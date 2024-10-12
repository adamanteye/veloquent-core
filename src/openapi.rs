use crate::*;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(
    paths(
        user::login_handler
    ),
    components(
        schemas(
            error::AppErrorResponse,
            user::LoginRequest, user::LoginResponse
        )
    ),
    tags(
        (name = "user", description = "用户管理")
    )
)]
pub(super) struct ApiDoc;
