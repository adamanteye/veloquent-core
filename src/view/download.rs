use super::*;
use entity::prelude::Upload;
use tokio::io::AsyncReadExt;
use utility::{UPLOAD_DIR, UUID_NIL};

/// 资源体
#[derive(prost::Message, ToSchema)]
pub struct Resource {
    /// 扩展名或文件类型
    ///
    /// `tag` = `1`
    #[prost(string, tag = "1")]
    pub typ: String,
    /// 数据
    ///
    /// `tag` = `2`
    #[prost(bytes, tag = "2")]
    #[schema(value_type = String, format = Binary)]
    pub data: Bytes,
}

/// 获取静态资源
///
/// 返回 protobuf 格式数据
#[utoipa::path(
    get,
    path = "/download/{id}",
    params(
        ("id" = Uuid, Path, description = "资源主键")
    ),
    responses(
        (status = 200, description = "获取成功", body = Resource),
        (status = 404, description = "获取失败", body = AppErrorResponse),
    ),
    tag = "static"
)]
#[instrument(skip(state, _payload))]
pub async fn download_handler(
    State(state): State<AppState>,
    _payload: JWTPayload,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let file = Upload::find_by_id(id).one(&state.conn).await?;
    let file = file.ok_or(AppError::NotFound(format!("cannot find file: [{}]", id)))?;
    if file.uuid == *UUID_NIL {
        return Err(AppError::BadRequest("empty content".to_string()));
    }
    let path = std::path::Path::new(UPLOAD_DIR.get().unwrap()).join(file.uuid.to_string());
    let typ = file.typ;
    let mut data = Vec::new();
    let mut file = tokio::fs::File::open(&path).await?;
    file.read_to_end(&mut data).await?;
    event!(Level::DEBUG, "download file: [{:?}]", path);
    Ok(Protobuf(Resource {
        typ,
        data: Bytes::from(data),
    }))
}
