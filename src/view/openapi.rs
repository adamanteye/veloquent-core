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
        contact::add_contact_handler, contact::get_contacts_handler,
        contact::get_pending_contacts_handler, contact::get_new_contacts_handler,
        contact::delete_contact_handler,
        contact::accept_contact_handler, contact::reject_contact_handler,
        message::send_msg_handler, message::get_msg_handler,
        group::get_group_handler, group::create_group_handler,
        group::delete_group_handler, group::list_group_handler,
        history::get_history_handler,
        avatar::upload_handler, avatar::upload_avatar_handler,
        download::download_handler,
    ),
    components(
        schemas(
            error::AppErrorResponse,
            user_register::RegisterProfile,
            user_profile::UserProfile, user_profile::UserProfileEdition,
            login::LoginRequest, login::LoginResponse,
            user_find::UserList,
            download::Resource, avatar::UploadRes,
            contact::ContactList, contact::Chat,
            message::MsgPost, message::MsgRes, message::Msg, message::ReadAt,
            group::GroupPost, group::GroupProfile, 
            history::History
        )
    ),
    tags(
        (name = "user", description = "用户管理"),
        (name = "contact", description = "好友管理"),
        (name = "msg", description = "消息发送"),
        (name = "group", description = "群聊管理"),
        (name = "static", description = "静态资源")
    )
)]
pub(super) struct ApiDoc;
