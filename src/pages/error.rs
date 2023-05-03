use std::{backtrace::Backtrace, fmt::Display, panic::Location};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;
use futures::Future;
use tracing::{Instrument, Span};
use tracing_error::SpanTrace;

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

#[macro_export]
macro_rules! internal_server_error {
    ($name:expr, $($field:tt)+) => {
        InternalServerError::throw(
            tracing::error_span!($name, $($field)*),
            &|| tracing::error!({ $($field)+ }, $name)
        )
    };
    ($name:expr) => {
        InternalServerError::throw(
            tracing::error_span!($name),
            &|| tracing::error!($name)
        )
    };
}

#[derive(Debug)]
pub struct FormatError(String);

impl Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}
impl std::error::Error for FormatError {}

#[derive(Debug)]
pub struct InternalServerError {
    inner_error: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    span_trace: SpanTrace,
    backtrace: Backtrace,
    caller: &'static Location<'static>,
}

impl InternalServerError {
    #[inline]
    #[track_caller]
    #[doc = "hidden"]
    pub fn throw(span: Span, error: &dyn Fn()) -> Self {
        let caller = Location::caller();

        // "throw" error
        (error)();
        // // FIXME: Send more info to sentry somehow
        // // FIXME: include context?? HOW?
        // tracing::error!(location=%caller, "{error}");

        Self {
            inner_error: None,
            span_trace: SpanTrace::new(span),
            backtrace: Backtrace::capture(),
            caller,
        }
    }

    #[inline]
    #[track_caller]
    pub fn wrap<F, T, E>(
        future: F,
        span: Span,
    ) -> impl Future<Output = Result<T, InternalServerError>>
    where
        F: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let caller = Location::caller();
        let backtrace = Backtrace::capture();

        async move {
            match Instrument::instrument(future, span.clone()).await {
                Ok(data) => Ok(data),
                Err(error) => Err({
                    let _span = span.enter();
                    InternalServerError::from_error_inner(error, caller, backtrace)
                }),
            }
        }
    }

    #[inline]
    #[track_caller]
    pub fn wrap_in_current_span<F, T, E>(
        future: F,
    ) -> impl Future<Output = Result<T, InternalServerError>>
    where
        F: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let caller = Location::caller();
        let backtrace = Backtrace::capture();

        async move {
            let result = Instrument::in_current_span(future).await;

            match result {
                Ok(data) => Ok(data),
                Err(error) => Err(InternalServerError::from_error_inner(
                    error, caller, backtrace,
                )),
            }
        }
    }

    #[inline]
    #[track_caller]
    pub fn from_error<E>(error: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::from_error_inner(error, Location::caller(), Backtrace::capture())
    }

    #[inline]
    fn from_error_inner<E>(
        error: E,
        caller: &'static Location<'static>,
        backtrace: Backtrace,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        // FIXME: Send more info to sentry somehow
        // FIXME: include context?? HOW?
        tracing::error!(location=%caller, "{error}");

        InternalServerError {
            inner_error: Some(Box::new(error)),
            span_trace: SpanTrace::capture(),
            backtrace,
            caller,
        }
    }

    fn inner_error(&self) -> String {
        match &self.inner_error {
            Some(error) => error.to_string(),
            None => String::from("None"),
        }
    }
}

impl Display for InternalServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "INTERNAL SERVER ERROR")?;
        writeln!(f, "{}", self.inner_error())?;
        writeln!(f, "Location: {}", self.caller)?;
        writeln!(f, "{}", self.span_trace)
    }
}
impl std::error::Error for InternalServerError {}

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
                    pre { code { "{self.backtrace}" } }
                }
            } else {
                rsx! {""}
            },
        )
        .into_response()
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
                pre { code { "{panic_info.info.backtrace}" } }
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
