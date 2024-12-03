use super::*;
use download::Resource;
use entity::{
    prelude::{Upload, User},
    upload, user,
};
use tokio::io::AsyncWriteExt;
use utility::{bytes_as_uuid, UPLOAD_DIR, UUID_NIL};

/// 上传用户头像
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/upload/avatar",
    request_body = Resource,
    responses(
        (status = 201, description = "上传成功")
    ),
    tag = "static"
))]
#[instrument(skip(state, avatar))]
pub async fn upload_avatar_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Protobuf(avatar): Protobuf<Resource>,
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
    let user = User::find_by_id(payload.id).one(&state.conn).await?;
    let user = user.ok_or(AppError::NotFound(format!(
        "cannot find user: [{}]",
        payload.id
    )))?;
    let mut user: user::ActiveModel = user.into();
    let uuid = save_file(&avatar, &state.conn).await?;
    user.avatar = ActiveValue::set(Some(uuid));
    User::update(user).exec(&state.conn).await?;
    event!(Level::INFO, "update user:avatar [{}:{}]", payload.id, uuid);
    Ok(StatusCode::CREATED.into_response())
}

#[cfg_attr(feature = "dev", derive(ToSchema))]
#[derive(Serialize)]
pub struct UploadRes {
    /// 扩展名或文件类型
    typ: String,
    /// 数据库中的键值
    uuid: Uuid,
}

/// 通用上传
#[cfg_attr(feature = "dev",
utoipa::path(
    post,
    path = "/upload",
    request_body = Resource,
    responses(
        (status = 201, description = "上传成功")
    ),
    tag = "static"
))]
#[instrument(skip(state, avatar))]
pub async fn upload_handler(
    State(state): State<AppState>,
    payload: JWTPayload,
    Protobuf(avatar): Protobuf<Resource>,
) -> Result<Json<UploadRes>, AppError> {
    let uuid = save_file(&avatar, &state.conn).await?;
    Ok(Json(UploadRes {
        typ: avatar.typ,
        uuid,
    }))
}

async fn save_file(r: &Resource, c: &DatabaseConnection) -> Result<Uuid, AppError> {
    let data = &r.data;
    let uuid = bytes_as_uuid(&data);
    if uuid.eq(&UUID_NIL) {
        Err(AppError::BadRequest("empty content".to_string()))
    } else {
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
                    typ: ActiveValue::set(r.typ.clone()),
                };
                Upload::insert(file).exec(c).await?;
            }
            Err(e) => {
                event!(Level::DEBUG, "create file error: [{}]", e); // 文件已经存在
                let file = Upload::find_by_id(uuid).one(c).await?;
                match file {
                    Some(_) => {} // 数据库当中有记录
                    None => {
                        event!(Level::ERROR, "cannot from database find file: [{}]", uuid);
                        let file = upload::ActiveModel {
                            uuid: ActiveValue::set(uuid),
                            typ: ActiveValue::set(r.typ.clone()),
                        };
                        Upload::insert(file).exec(c).await?;
                    }
                }
            }
        };
        Ok(uuid)
    }
}
