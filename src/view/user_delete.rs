use super::*;
use entity::prelude::User;

/// 获取用户个人信息
#[utoipa::path(delete, path = "/user/profile", responses((status = 204, description = "删除成功")), tag = "user")]
#[instrument(skip(state))]
pub async fn delete_user_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Response, AppError> {
    let _: DeleteResult = User::delete_by_id(payload.id).exec(&state.conn).await?;
    event!(Level::INFO, "delete user [{}]", payload.id);
    Ok(StatusCode::NO_CONTENT.into_response())
}
