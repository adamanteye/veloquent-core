use super::view::*;
use crate::*;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(
    paths(
        login::login_handler,
        user_profile::get_self_profile,
    ),
    components(
        schemas(
            error::AppErrorResponse,
            login::LoginRequest, login::LoginResponse,
            user_profile::UserProfile,
        )
    ),
    tags(
        (name = "user", description = "用户管理")
    )
)]
pub(super) struct ApiDoc;
