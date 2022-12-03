use axum::{
    body::Body,
    http::{Request, Uri},
    response::{IntoResponse, Response},
    Extension,
};
use tower_http::request_id::RequestId;
use tracing::error;

use crate::{
    middleware::catch_panic::CaughtPanic,
    templates::error::{InternalServerError, NotFound},
};

pub async fn not_found(Extension(id): Extension<RequestId>, uri: Uri) -> Response {
    NotFound::response(uri.path(), id)
}

pub async fn static_not_found<E>(req: Request<Body>) -> Result<Response, E> {
    Ok(NotFound::response(
        &format!("/static{}", req.uri().path()),
        req.extensions()
            .get::<RequestId>()
            .expect("x-header-id should be set")
            .clone(),
    ))
}

#[tracing::instrument]
pub async fn internal_server_error<E: std::error::Error>(
    Extension(id): Extension<RequestId>,
    error: E,
) -> Response {
    error!(%error, "ServeDir encountered IO error"); // FIXME:

    InternalServerError::from_error(&error, id).into_response()
}

#[tracing::instrument(skip(info))]
pub fn internal_server_error_panic(request_id: RequestId, info: CaughtPanic) -> Response {
    if let Some(message) = info.payload_str() {
        error!(%message, "Service panicked");
    } else {
        error!("Service panicked but panic info was not a &str or String");
    }

    InternalServerError::from_panic(request_id, info).into_response()
}
