use std::any::Any;

use axum::{
    body::Body,
    http::{Request, Uri},
    response::Response,
};
use tower_http::request_id::RequestId;
use tracing::error;

use crate::templates::error::{InternalServerError, NotFound};

pub async fn not_found(id: axum::extract::Extension<RequestId>, uri: Uri) -> Response {
    NotFound::response(uri.path(), id.0)
}

pub async fn not_found_service<E>(req: Request<Body>) -> Result<Response, E> {
    Ok(NotFound::response(
        req.uri().path(),
        req.extensions()
            .get::<RequestId>()
            .expect("x-header-id should be set")
            .clone(),
    ))
}

pub async fn internal_server_error<E: std::error::Error>(
    id: axum::extract::Extension<RequestId>,
    error: E,
) -> Response {
    error!(%error, "ServeDir encountered IO error"); // FIXME:

    InternalServerError::response(error.to_string(), id.0)
}

pub fn internal_server_error_panic(
    request_id: RequestId,
    panic: Box<dyn Any + Send + 'static>,
) -> Response {
    let details = if let Some(s) = panic.downcast_ref::<String>() {
        tracing::error!("Service panicked: {}", s);

        s.as_str()
    } else if let Some(s) = panic.downcast_ref::<&str>() {
        tracing::error!("Service panicked: {}", s);

        s
    } else {
        tracing::error!("Service panicked but panic info was not a &str or String");

        "Unknown panic message"
    };

    InternalServerError::response(format!("PANIC: {details}"), request_id)
}
