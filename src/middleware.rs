use std::panic::AssertUnwindSafe;

use axum::{
    body::{Body, BoxBody},
    extract::{Host, State},
    http::{
        uri::{Authority, Parts, Scheme},
        Request, Response, Uri,
    },
    middleware::Next,
    response::{IntoResponse, Redirect},
    Extension,
};
use futures::FutureExt;
use tower_http::request_id::RequestId;
use tracing::trace;

use crate::routes::error::internal_server_error_panic;

#[tracing::instrument(skip(req, next))]
pub async fn redirect_to_domain(
    State(expected_host): State<Authority>,
    Host(host): Host,
    req: Request<Body>,
    next: Next<Body>,
) -> Response<BoxBody> {
    if host == expected_host.as_ref() {
        next.run(req).await
    } else {
        let mut parts = Parts::default();
        // Inherit path and query from request
        parts.path_and_query = req.uri().path_and_query().cloned();

        parts.authority = Some(expected_host);
        parts.scheme = Some(if cfg!(debug_assertions) {
            Scheme::HTTP
        } else {
            Scheme::HTTPS
        });

        trace!("URI authority did not match configured HOST");

        Redirect::permanent(
            &Uri::from_parts(parts)
                .expect("redirect uri should be a valid uri")
                .to_string(),
        )
        .into_response()
    }
}

/// Re-implementation of [`tower_http::catch_panic::CatchPanicLayer`] to allow
/// for capturing the Request ID for panic handlers
pub async fn catch_panic(
    Extension(request_id): Extension<RequestId>,
    req: Request<Body>,
    next: Next<Body>,
) -> Response<BoxBody> {
    // Catch panic before return of future
    let panic_err = match std::panic::catch_unwind(AssertUnwindSafe(move || next.run(req))) {
        // Catch panic while polling future
        Ok(future) => match AssertUnwindSafe(future).catch_unwind().await {
            Ok(response) => return response,
            Err(panic_err) => panic_err,
        },
        Err(panic_err) => panic_err,
    };

    // TODO: allow config I guess
    internal_server_error_panic(request_id, panic_err)
}
