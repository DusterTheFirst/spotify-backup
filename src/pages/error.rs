use std::{error::Error, fmt::Display};

use crate::middleware::{catch_panic::CaughtPanic, RequestMetadata};

use super::Page;

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

fn error_sources(error: &dyn Error) -> Box<dyn Iterator<Item = String>> {
    if let Some(source) = error.source() {
        return Box::new(std::iter::once(source.to_string()).chain(error_sources(source)));
    }

    Box::new(std::iter::empty())
}

pub fn dyn_error(error: &dyn Error, request_meta: RequestMetadata) -> impl IntoResponse {
    let sources = error_sources(&error);
    let error = error.to_string();

    self::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        request_meta,
        if cfg!(debug_assertions) {
            rsx! {
                div {
                    error
                }
                div {
                    for (i, source) in sources.enumerate() {
                        div { key: "{i}", source }
                    }
                }
            }
        } else {
            rsx! { "" }
        },
    )
}

pub fn panic_error(panic_info: CaughtPanic, request_meta: RequestMetadata) -> impl IntoResponse {
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        request_meta,
        if cfg!(debug_assertions) {
            rsx! {
                div { "The application panicked." }
                div {
                    if let Some(message) = panic_info.payload_str() {
                        rsx! { "Message: {message}" }
                    } else {
                        rsx! { "Unknown panic message" }
                    }
                }
                div {
                    if let Some(location) = panic_info.location() {
                        rsx! { "Location: {location}" }
                    }
                }

                h2 { "Span Trace" }
                pre { panic_info.span_trace().to_string() }

                h2 { "Backtrace" }
                pre { panic_info.backtrace().to_string() }
            }
        } else {
            rsx! { "" }
        },
    )
}

pub fn error(
    status: StatusCode,
    request_meta: RequestMetadata,
    body: LazyNodes<'static, 'static>,
) -> (StatusCode, Page<'static>) {
    let status_code = status.as_str();
    let status_reason = status.canonical_reason().unwrap_or("Unknown Error");

    (
        status,
        Page {
            title: rsx! { "{status_code} ({status_reason})" },
            head: Some(rsx! {
                style { include_str!("error.css") }
            }),
            content: rsx! {
                header {
                    h1 { "{status_code} | {status_reason}" }
                }
                main {
                    body
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
        },
    )
}
