use std::any::Any;

use axum::{
    body::Body,
    http::{Request, Uri},
    response::Response,
};
use tracing::error;

use crate::templates::error::{InternalServerError, NotFound};

pub async fn not_found(uri: Uri) -> Response {
    NotFound::response(uri.path())
}

pub async fn not_found_service<E>(req: Request<Body>) -> Result<Response, E> {
    Ok(NotFound::response(req.uri().path()))
}

pub async fn internal_server_error<E: std::error::Error>(error: E) -> Response {
    error!(%error, "ServeDir encountered IO error"); // FIXME:

    InternalServerError::response(error.to_string())
}

pub fn internal_server_error_panic(panic: Box<dyn Any + Send + 'static>) -> Response {
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

    InternalServerError::response(format!("PANIC: {details}"))
}
