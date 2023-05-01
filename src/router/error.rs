use std::fmt::Display;

use axum::{
    body::Body,
    extract::OriginalUri,
    http::Request,
    response::{IntoResponse, Response},
};
use tracing::error;

use crate::pages;

use super::middleware::catch_panic::CaughtPanic;

pub async fn not_found(OriginalUri(uri): OriginalUri) -> Response {
    pages::not_found(uri.path())
}

pub async fn static_not_found(req: Request<Body>) -> Response {
    pages::not_found(
        req.extensions()
            .get::<OriginalUri>()
            .expect("OriginalUri extractor should exist on router")
            .0
            .path(),
    )
}

pub fn internal_server_error_panic(info: CaughtPanic) -> Response {
    let location: &dyn Display = match info.location() {
        Some(location) => location as &dyn Display,
        None => &"Unknown" as &dyn Display,
    };

    if let Some(panic) = info.payload_str() {
        error!(%panic, %location, "service panicked");
    } else {
        error!("service panicked but panic info was not a &str or String");
    }

    pages::panic_error(info).into_response()
}
