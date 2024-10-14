use super::*;
use crate::error;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(
    paths(
        login::login_handler,
        user_register::register_handler,
        user_profile::get_self_profile_handler,
        user_delete::delete_user_handler,
    ),
    components(
        schemas(
            error::AppErrorResponse,
            user_register::RegisterProfile,
            login::LoginRequest, login::LoginResponse,
            user_profile::UserProfile,
        )
    ),
    tags(
        (name = "user", description = "用户管理")
    )
)]
pub(super) struct ApiDoc;
