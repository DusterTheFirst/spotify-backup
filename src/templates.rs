use std::fmt::Debug;

use askama::Template;
use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::error;

pub mod error;

#[tracing::instrument(level="trace", skip_all, fields(template = std::any::type_name::<T>()))]
fn into_response<T: Template + Debug>(t: T) -> Response {
    match t.render() {
        Ok(body) => {
            let headers = [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static(T::MIME_TYPE),
            )];

            (headers, body).into_response()
        }
        Err(error) => {
            // TODO: error handling page
            error!(
                ?error,
                "encountered error converting template into response"
            );

            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
