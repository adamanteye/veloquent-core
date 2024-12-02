use super::*;
use entity::prelude::User;

/// 删除用户自己
#[cfg_attr(feature = "dev", utoipa::path(delete, path = "/user/profile", responses((status = 204, description = "删除成功")), tag = "user"))]
#[instrument(skip(state))]
pub async fn delete_user_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
) -> Result<Response, AppError> {
    let res: DeleteResult = User::delete_by_id(payload.id).exec(&state.conn).await?;
    if res.rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "cannot delete user [{}]",
            payload.id
        )));
    }
    event!(Level::INFO, "delete user [{}]", payload.id);
    Ok(StatusCode::NO_CONTENT.into_response())
}
