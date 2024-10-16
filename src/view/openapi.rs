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
        user_find::find_user_handler,
        avatar::upload_avatar_handler,
    ),
    components(
        schemas(
            error::AppErrorResponse,
            user_register::RegisterProfile,
            user_profile::UserProfile,
            login::LoginRequest, login::LoginResponse,
            user_find::UserList,
        )
    ),
    tags(
        (name = "user", description = "用户管理")
    )
)]
pub(super) struct ApiDoc;
