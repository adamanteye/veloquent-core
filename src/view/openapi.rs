use super::*;
use crate::error;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(
    paths(
        login::login_handler,
        user_register::register_handler,
        user_profile::get_profile_handler, user_profile::update_profile_handler,
        user_delete::delete_user_handler,
        user_find::find_user_handler,
        avatar::upload_avatar_handler,
        download::download_handler,
    ),
    components(
        schemas(
            error::AppErrorResponse,
            user_register::RegisterProfile,
            user_profile::UserProfile, user_profile::UserProfileEdition,
            login::LoginRequest, login::LoginResponse,
            user_find::UserList,
            download::Resource,
        )
    ),
    tags(
        (name = "user", description = "用户管理"),
        (name = "static", description = "静态资源")
    )
)]
pub(super) struct ApiDoc;
