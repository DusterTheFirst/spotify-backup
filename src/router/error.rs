use axum::{
    body::Body,
    extract::OriginalUri,
    http::Request,
    response::{IntoResponse, Response},
    RequestExt,
};
use tracing::error;

use crate::pages;

use super::middleware::{catch_panic::CaughtPanic, request_metadata::RequestMetadata};

#[tracing::instrument(level = "trace", skip(request_metadata))]
pub async fn not_found(
    request_metadata: RequestMetadata,
    OriginalUri(uri): OriginalUri,
) -> Response {
    pages::not_found(uri.path(), &request_metadata)
}

#[tracing::instrument(level = "trace", skip_all)]
pub async fn static_not_found(mut req: Request<Body>) -> Response {
    let request_meta = req
        .extract_parts::<RequestMetadata>()
        .await
        .expect("RequestMetadata should be infallible");

    pages::not_found(
        req.extensions()
            .get::<OriginalUri>()
            .expect("OriginalUri extractor should exist on router")
            .0
            .path(),
        &request_meta,
    )
}

#[tracing::instrument(level = "trace", skip(request_metadata))]
pub async fn internal_server_error<E: std::error::Error>(
    request_metadata: RequestMetadata,
    error: E,
) -> Response {
    error!(%error, "ServeDir encountered IO error"); // FIXME:

    pages::dyn_error(&error, &request_metadata).into_response()
}

#[tracing::instrument(level = "trace", skip_all)]
pub fn internal_server_error_panic(
    request_metadata: RequestMetadata,
    info: CaughtPanic,
) -> Response {
    if let Some(panic) = info.payload_str() {
        error!(%panic, "service panicked");
    } else {
        error!("service panicked but panic info was not a &str or String");
    }

    pages::panic_error(info, &request_metadata).into_response()
}
