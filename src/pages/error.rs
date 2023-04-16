use std::error::Error;

use crate::router::middleware::catch_panic::CaughtPanic;

use super::Page;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;

pub fn not_found(path: &str) -> Response {
    self::error(
        StatusCode::NOT_FOUND,
        rsx! {
            div {
                code { path }
                " not found"
            }
        },
    )
    .into_response()
}

fn error_sources(error: &dyn Error) -> Box<dyn Iterator<Item = String>> {
    if let Some(source) = error.source() {
        return Box::new(std::iter::once(source.to_string()).chain(error_sources(source)));
    }

    Box::new(std::iter::empty())
}

pub struct EyreReport {
    report: color_eyre::Report,
}

impl From<color_eyre::Report> for EyreReport {
    fn from(value: color_eyre::Report) -> Self {
        Self { report: value }
    }
}

// TODO: way to wrap handlers to always coerce errors to this page?
// FIXME: error traces somehow
// FIXME: use eyre?
impl IntoResponse for EyreReport {
    fn into_response(self) -> Response {
        tracing::error!(%self.report, "encountered an error serving a page");

        let sources = error_sources(self.report.as_ref());
        let error = self.report.to_string();

        self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            if cfg!(debug_assertions) {
                rsx! {
                    div { error }
                    ul {
                        for (i , source) in sources.enumerate() {
                            li { key: "{i}", source }
                        }
                    }
                }
            } else {
                rsx! {""}
            },
        )
        .into_response()
    }
}

pub fn dyn_error(error: &dyn Error) -> impl IntoResponse {
    tracing::error!(%error, "encountered an error serving page");

    let sources = error_sources(&error);
    let error = error.to_string();

    self::error(
        StatusCode::INTERNAL_SERVER_ERROR,
        if cfg!(debug_assertions) {
            rsx! {
                div { error }
                ul {
                    for (i , source) in sources.enumerate() {
                        li { key: "{i}", source }
                    }
                }
            }
        } else {
            rsx! {""}
        },
    )
}

pub fn panic_error(panic_info: CaughtPanic) -> impl IntoResponse {
    self::error(
        StatusCode::INTERNAL_SERVER_ERROR,
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
            rsx! {""}
        },
    )
}

fn error<'a>(status: StatusCode, body: LazyNodes<'a, 'a>) -> (StatusCode, Page<'a>) {
    let status_code = status.as_u16();
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
                // TODO: what do with? why even have?
                // footer {
                //     section {
                //         h4 { "Request ID" }
                //         code { request_meta.request_id }
                //     }
                //     section {
                //         h4 { "Region" }
                //         request_meta.region
                //     }
                //     section {
                //         h4 { "Server" }
                //         code { "{request_meta.server.name} {request_meta.server.version}" }
                //         code { "(commit " a { href:request_meta.server.source, target:"_blank", request_meta.server.commit } ")" }
                //     }
                // },
            },
        },
    )
}
