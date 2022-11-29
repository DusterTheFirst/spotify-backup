use super::into_response;

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use git_version::git_version;

#[derive(Template, Debug)]
#[template(path = "error/404.html")]
pub struct NotFound<'s> {
    pub(crate) path: &'s str,
}

impl<'s> NotFound<'s> {
    pub fn response(path: &str) -> Response {
        (StatusCode::NOT_FOUND, into_response(NotFound { path })).into_response()
    }
}

#[derive(Template, Debug)]
#[template(path = "error/500.html")]
pub struct InternalServerError {
    pub(crate) error_message: Option<String>,
}

impl InternalServerError {
    pub fn response(error_message: impl Into<String>) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            into_response(InternalServerError {
                error_message: if cfg!(debug_assertions) {
                    Some(error_message.into())
                } else {
                    None
                },
            }),
        )
            .into_response()
    }
}
