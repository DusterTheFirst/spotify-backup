use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;

use crate::router::middleware::catch_panic::{CaughtPanic, Location};

use super::Page;

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

pub struct ErrorPage {
    report: color_eyre::Report,
    caller: Location,
}

impl From<color_eyre::Report> for ErrorPage {
    #[track_caller]
    fn from(value: color_eyre::Report) -> Self {
        Self {
            report: value,
            caller: std::panic::Location::caller().into(),
        }
    }
}

impl IntoResponse for ErrorPage {
    fn into_response(self) -> Response {
        let chain = self
            .report
            .chain()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        // FIXME: Send more info to sentry somehow
        tracing::error!(?chain, %self.caller, "encountered an error serving a page");

        self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            if cfg!(debug_assertions) {
                rsx! {
                    dioxus_ansi::preformatted_ansi {
                        // FIXME: Allocation :(
                        ansi_text: format!("{:?}", self.report)
                    }
                }
            } else {
                rsx! {""}
            },
        )
        .into_response()
    }
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
            content: rsx! {
                header {
                    class: "error_message",
                    "{status_code} | {status_reason}"
                }
                main {
                    body
                },
                nav {
                    a { href: "/", "return home" }
                },
            },
        },
    )
}
