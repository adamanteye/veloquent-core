use super::*;
use entity::{
    prelude::{Upload, User},
    upload, user,
};
use tokio::io::AsyncWriteExt;
use utility::{bytes_as_uuid, UPLOAD_DIR, UUID_NIL};

/// 上传用户头像
#[utoipa::path(
    post,
    path = "/upload/avatar",
    request_body = Resource,
    responses(
        (status = 201, description = "上传成功")
    ),
    tag = "static"
)]
#[instrument(skip(state, avatar))]
pub async fn upload_avatar_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Protobuf(avatar): Protobuf<super::download::Resource>,
) -> Result<Response, AppError> {
    if avatar.typ.is_empty() {
        return Err(AppError::BadRequest("empty type".to_string()));
    }
    if avatar.typ.ne("png") && avatar.typ.ne("jpg") {
        return Err(AppError::BadRequest(format!(
            "unsupported type: [{}]",
            avatar.typ
        )));
    }
    let data = avatar.data;
    let uuid = bytes_as_uuid(&data);
    if uuid.eq(&UUID_NIL) {
        Err(AppError::BadRequest("empty content".to_string()))
    } else {
        let user = User::find_by_id(payload.id).one(&state.conn).await?;
        let user = user.ok_or(AppError::NotFound(format!(
            "cannot find user: [{}]",
            payload.id
        )))?;
        let mut user: user::ActiveModel = user.into();
        let file = tokio::fs::File::create_new(
            std::path::Path::new(&UPLOAD_DIR.get().unwrap()).join(uuid.to_string()),
        )
        .await;
        match file {
            Ok(mut f) => {
                f.write_all(&data).await?;
                event!(Level::INFO, "write file: [{}]", uuid);
                let file = upload::ActiveModel {
                    uuid: ActiveValue::set(uuid),
                    typ: ActiveValue::set(avatar.typ),
                };
                Upload::insert(file).exec(&state.conn).await?;
            }
            Err(e) => {
                event!(Level::DEBUG, "create file error: [{}]", e);
                let file = Upload::find_by_id(uuid).one(&state.conn).await?;
                match file {
                    Some(_) => {}
                    None => {
                        event!(Level::ERROR, "cannot from database find file: [{}]", uuid);
                        let file = upload::ActiveModel {
                            uuid: ActiveValue::set(uuid),
                            typ: ActiveValue::set(avatar.typ),
                        };
                        Upload::insert(file).exec(&state.conn).await?;
                    }
                }
            }
        };
        user.avatar = ActiveValue::set(Some(uuid));
        let _ = User::update(user).exec(&state.conn).await?;
        event!(Level::INFO, "update user:avatar [{}:{}]", payload.id, uuid);
        Ok(StatusCode::CREATED.into_response())
    }
}
