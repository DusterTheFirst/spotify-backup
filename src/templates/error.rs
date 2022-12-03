use std::error::Error;

use crate::middleware::catch_panic::CaughtPanic;

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
    details: Option<InternalServerErrorDetails>,
    request_id: RequestId,
}

#[derive(Debug)]
enum InternalServerErrorDetails {
    Error(InternalServerErrorError),
    Panic(InternalServerErrorPanic),
}

#[derive(Template, Debug)]
#[template(path = "error/500-error.html")]
struct InternalServerErrorError {
    error_message: String,
    source: Vec<String>,
}

#[derive(Template, Debug)]
#[template(path = "error/500-panic.html")]
struct InternalServerErrorPanic {
    panic: CaughtPanic,
}

fn error_sources(error: &dyn Error) -> Vec<String> {
    if let Some(source) = error.source() {
        let mut sources = error_sources(source);
        sources.push(source.to_string());
        return sources;
    }

    vec![]
}

impl InternalServerError {
    pub fn from_error(error: &dyn Error, request_id: RequestId) -> Self {
        InternalServerError {
            details: cfg!(debug_assertions).then_some(InternalServerErrorDetails::Error(
                InternalServerErrorError {
                    error_message: error.to_string(),
                    source: error_sources(&error),
                },
            )),

            request_id,
        }
    }

    pub fn from_panic(request_id: RequestId, panic_info: CaughtPanic) -> Self {
        InternalServerError {
            details: cfg!(debug_assertions).then_some(InternalServerErrorDetails::Panic(
                InternalServerErrorPanic { panic: panic_info },
            )),
            request_id,
        }
    }
}

impl IntoResponse for InternalServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, into_response(self)).into_response()
    }
}
