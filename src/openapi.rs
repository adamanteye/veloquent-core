use crate::*;
use utoipa::OpenApi;

#[doc(hidden)]
#[derive(OpenApi)]
#[openapi(components(schemas(error::AppErrorResponse)))]
pub(super) struct ApiDoc;
