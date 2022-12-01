use super::into_response;

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use git_version::git_version;
use tower_http::request_id::RequestId;

#[derive(Template, Debug)]
#[template(path = "error/404.html")]
pub struct NotFound<'s> {
    path: &'s str,
    request_id: RequestId,
}

impl<'s> NotFound<'s> {
    pub fn response(path: &str, request_id: RequestId) -> Response {
        (
            StatusCode::NOT_FOUND,
            into_response(NotFound { path, request_id }),
        )
            .into_response()
    }
}

#[derive(Template, Debug)]
#[template(path = "error/500.html")]
pub struct InternalServerError {
    error_message: Option<String>,
    request_id: RequestId,
}

impl InternalServerError {
    pub fn response(error_message: impl Into<String>, request_id: RequestId) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            into_response(InternalServerError {
                error_message: if cfg!(debug_assertions) {
                    Some(error_message.into())
                } else {
                    None
                },
                request_id,
            }),
        )
            .into_response()
    }
}
