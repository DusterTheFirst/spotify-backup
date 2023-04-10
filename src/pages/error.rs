use std::error::Error;

use crate::middleware::{catch_panic::CaughtPanic, RequestMetadata};

use super::{into_response, Page};

use askama::Template;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;

#[derive(Template, Debug)]
#[template(path = "error/404.html")]
pub struct NotFound<'s> {
    path: &'s str,
    request_meta: RequestMetadata,
}

impl<'s> NotFound<'s> {
    pub fn response(path: &str, request_meta: RequestMetadata) -> Response {
        (
            StatusCode::NOT_FOUND,
            into_response(NotFound { path, request_meta }),
        )
            .into_response()
    }
}

#[derive(Template, Debug)]
#[template(path = "error/500.html")]
pub struct InternalServerError {
    details: Option<InternalServerErrorDetails>,
    request_meta: RequestMetadata,
}

#[derive(Debug)]
enum InternalServerErrorDetails {
    Error(InternalServerErrorError),
    Panic(InternalServerErrorPanic),
}

#[derive(Template, Debug)]
#[template(path = "error/500-error.html")]
struct InternalServerErrorError {
    error_message: String, // TODO:
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
    pub fn from_error(error: &dyn Error, request_meta: RequestMetadata) -> Self {
        InternalServerError {
            details: cfg!(debug_assertions).then_some(InternalServerErrorDetails::Error(
                InternalServerErrorError {
                    error_message: error.to_string(),
                    source: error_sources(&error),
                },
            )),
            request_meta,
        }
    }

    pub fn from_panic(panic_info: CaughtPanic, request_meta: RequestMetadata) -> Self {
        InternalServerError {
            details: cfg!(debug_assertions).then_some(InternalServerErrorDetails::Panic(
                InternalServerErrorPanic { panic: panic_info },
            )),
            request_meta,
        }
    }
}

impl IntoResponse for InternalServerError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, into_response(self)).into_response()
    }
}

pub fn error(
    (status_code, status_message): (u16, &'static str),
    main: Element<'static>,
    request_meta: RequestMetadata,
) -> Page<'static> {
    Page {
        title: rsx! { "{status_code} ({status_message})" },
        head: None,
        content: rsx! {
            header {
                h1 { "{status_code} | {status_message}" }
            }
            main {
                main
            },
            nav {
                a { href: "/", "return home" }
            },
            footer {
                section {
                    h4 { "Request ID" }
                    code { request_meta.request_id }
                }
                section {
                    h4 { "Region" }
                    request_meta.region
                }
                section {
                    h4 { "Server" }
                    code { "{request_meta.server.name} {request_meta.server.version}" }
                    code { "(commit " a { href:request_meta.server.source, target:"_blank", request_meta.server.commit } ")" }
                }
            },
        },
    }
}
