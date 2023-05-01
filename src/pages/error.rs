use std::{
    backtrace::{Backtrace, BacktraceStatus},
    fmt::{Debug, Display},
    panic::Location,
};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;
use tracing::{field::ValueSet, Span};
use tracing_error::{InstrumentResult, SpanTrace};

use crate::router::middleware::catch_panic::CaughtPanic;

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

pub struct ClientError {
    message: String,
}

impl ClientError {
    pub fn new(message: String) -> Self {
        // TODO: log this?
        Self { message }
    }
}

impl IntoResponse for ClientError {
    fn into_response(self) -> Response {
        self::error(
            StatusCode::BAD_REQUEST,
            rsx! {
                div { self.message }
            },
        )
        .into_response()
    }
}

pub struct InternalServerError {
    inner_error: Option<Box<dyn std::error::Error>>,
    span_trace: SpanTrace,
    backtrace: Backtrace,
    caller: &'static Location<'static>,
}

impl InternalServerError {
    #[track_caller]
    pub fn new(error_span: tracing::span::Span) -> Self {
        let caller = Location::caller();
        let backtrace = Backtrace::capture();

        error_span.in_scope(|| {
            Self {
                inner_error: None,
                span_trace: SpanTrace::capture(),
                backtrace,
                caller,
            }
            .throw()
        })
    }

    fn inner_error(&self) -> String {
        match &self.inner_error {
            Some(error) => error.to_string(),
            None => String::from("None"),
        }
    }

    #[inline(always)]
    fn throw(self) -> Self {
        // FIXME: Send more info to sentry somehow
        // FIXME: include context?? HOW?
        // Look at implementation of color_eyre handler?
        tracing::error!(caller=%self.caller, error=%self.inner_error(), "encountered an error serving a page");

        self
    }
}

pub struct FormatError(String);

impl Debug for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl std::error::Error for FormatError {}

impl<E> From<E> for InternalServerError
where
    E: std::error::Error + 'static,
{
    #[track_caller]
    fn from(value: E) -> Self {
        Self {
            inner_error: Some(Box::new(value)),
            span_trace: SpanTrace::capture(),
            backtrace: Backtrace::capture(),
            caller: Location::caller(),
        }
        .throw()
    }
}

pub trait InstrumentErrorCustom<T> {
    type Instrumented;

    fn instrument_error(self, span: Span) -> Result<T, Self::Instrumented>;
}

impl<T, R> InstrumentErrorCustom<T> for R
where
    R: InstrumentResult<T>,
{
    type Instrumented = R::Instrumented;

    fn instrument_error(self, span: Span) -> Result<T, Self::Instrumented> {
        span.in_scope(|| InstrumentResult::<T>::in_current_span(self))
    }
}

impl IntoResponse for InternalServerError {
    fn into_response(self) -> Response {
        self::error(
            StatusCode::INTERNAL_SERVER_ERROR,
            if cfg!(debug_assertions) {
                rsx! {
                    h3 { "Error" }
                    pre { code { self.inner_error() } }

                    h3 { "Source" }
                    pre { code { "{self.caller}" } }

                    h3 { "Span Trace" }
                    dioxus_ansi::preformatted_ansi {
                        // FIXME: Allocation :(
                        ansi_text: color_spantrace::colorize(&self.span_trace).to_string()
                    }

                    h3 { "Backtrace" }
                    backtrace(self.backtrace)
                }
            } else {
                rsx! {""}
            },
        )
        .into_response()
    }
}

pub fn backtrace<'a>(backtrace: Backtrace) -> LazyNodes<'a, 'a> {
    rsx! {
        pre {
            code {
                match backtrace.status() {
                    BacktraceStatus::Captured => rsx!{
                        "{backtrace}"
                    },
                    BacktraceStatus::Unsupported => rsx! {
                        "capturing backtraces is unsupported"
                    },
                    BacktraceStatus::Disabled => rsx! {
                        "capturing of backtraces is disabled, enable with RUST_BACKTRACE=1"
                    },
                    _ => rsx! {
                        "backtrace is in an unknown state: {backtrace.status():?}"
                    }
                }
            }
        }
    }
}

pub fn panic_error(panic_info: CaughtPanic) -> Response {
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
                dioxus_ansi::preformatted_ansi {
                    // FIXME: Allocation :(
                    ansi_text: color_spantrace::colorize(&panic_info.info.span_trace).to_string()
                }

                h2 { "Backtrace" }
                // FIXME: cringe clone
                backtrace(panic_info.info.backtrace)
            }
        } else {
            rsx! {""}
        },
    )
    .into_response()
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
