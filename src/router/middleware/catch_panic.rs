use std::{
    any::Any,
    backtrace::Backtrace,
    fmt,
    panic::AssertUnwindSafe,
    pin::Pin,
    sync::{Arc, Mutex},
};

use axum::{
    body::{Body, BoxBody},
    extract::State,
    http::{Request, Response},
    middleware::{FromFnLayer, Next},
};
use futures::{Future, FutureExt};
use tracing_error::SpanTrace;

use super::request_metadata::RequestMetadata;

#[derive(Debug)]
pub struct CaughtPanic {
    payload: Box<dyn Any + Send + 'static>,
    info: PanicInfo,
}

impl CaughtPanic {
    pub fn payload(&self) -> &Box<dyn Any + Send + 'static> {
        &self.payload
    }

    pub fn payload_str(&self) -> Option<&str> {
        if let Some(s) = self.payload.downcast_ref::<String>() {
            Some(s.as_str())
        } else if let Some(s) = self.payload.downcast_ref::<&str>() {
            Some(s)
        } else {
            None
        }
    }

    pub fn location(&self) -> Option<&Location> {
        self.info.location.as_ref()
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.info.backtrace
    }

    pub fn span_trace(&self) -> &SpanTrace {
        &self.info.span_trace
    }
}

#[derive(Debug)]
pub struct PanicInfo {
    location: Option<Location>,
    backtrace: Backtrace,
    span_trace: SpanTrace,
}

#[derive(Debug)]
pub struct Location {
    file: String,
    line: u32,
    column: u32,
}

impl Location {
    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn line(&self) -> u32 {
        self.line
    }

    pub fn column(&self) -> u32 {
        self.column
    }
}

impl<'a> From<&'a std::panic::Location<'a>> for Location {
    fn from(location: &'a std::panic::Location<'a>) -> Self {
        Self {
            file: location.file().to_string(),
            line: location.line(),
            column: location.column(),
        }
    }
}

impl fmt::Display for Location {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}:{}", self.file, self.line, self.column)
    }
}

#[derive(Clone)]
pub struct CatchPanicState {
    panic_info: Arc<Mutex<Option<PanicInfo>>>,
    handler: CatchPanicHandler,
}

#[tracing::instrument]
pub fn catch_panic_layer<T>(
    handler: CatchPanicHandler,
) -> FromFnLayer<CatchPanicFn, CatchPanicState, T> {
    let panic_info: Arc<Mutex<Option<PanicInfo>>> = Arc::new(Mutex::new(None));

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new({
        let last_panic = panic_info.clone();

        move |info| {
            let backtrace = Backtrace::force_capture();
            let span_trace = SpanTrace::capture();

            *last_panic.lock().expect("mutex should not be poisoned") = Some(PanicInfo {
                location: info.location().map(Location::from),
                backtrace,
                span_trace,
            });

            previous_hook(info)
        }
    }));

    axum::middleware::from_fn_with_state(
        CatchPanicState {
            panic_info,
            handler,
        },
        catch_panic,
    )
}

type CatchPanicFn = fn(
    State<CatchPanicState>,
    RequestMetadata,
    Request<Body>,
    Next<Body>,
) -> Pin<Box<dyn Future<Output = Response<BoxBody>> + Send + 'static>>;

type CatchPanicHandler = fn(RequestMetadata, CaughtPanic) -> Response<BoxBody>;

/// Re-implementation of [`tower_http::catch_panic::CatchPanicLayer`] to allow
/// for capturing the Request ID for panic handlers as well as other fields from
/// [`PanicInfo`](std::panic::PanicInfo)
fn catch_panic(
    State(CatchPanicState {
        panic_info,
        handler,
    }): State<CatchPanicState>,
    request_meta: RequestMetadata,
    req: Request<Body>,
    next: Next<Body>,
) -> Pin<Box<dyn Future<Output = Response<BoxBody>> + Send + 'static>> {
    Box::pin(async move {
        // Catch panic before return of future
        let panic_payload = match std::panic::catch_unwind(AssertUnwindSafe(move || next.run(req)))
        {
            // Catch panic while polling future
            Ok(future) => match AssertUnwindSafe(future).catch_unwind().await {
                Ok(response) => return response,
                Err(panic_err) => panic_err,
            },
            Err(panic_err) => panic_err,
        };

        let panic_info = panic_info.lock().expect("mutex should not be poisoned").take().expect(
            "panic_info should be filled with new panic information by the time catch_panic runs",
        );

        handler(
            request_meta,
            CaughtPanic {
                payload: panic_payload,
                info: panic_info,
            },
        )
    })
}
