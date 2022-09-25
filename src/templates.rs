use askama::Template;
use axum::{
    body::Body,
    http::{self, Request, StatusCode},
    response::{IntoResponse, Response},
};

#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFound<'s> {
    path: &'s str,
}

pub fn not_found(path: &str) -> NotFound<'_> {
    NotFound { path }
}

pub async fn not_found_service<E>(request: Request<Body>) -> Result<Response, E> {
    Ok(not_found(request.uri().path()).into_response())
}

impl IntoResponse for NotFound<'_> {
    fn into_response(self) -> Response {
        (StatusCode::NOT_FOUND, into_response(&self)).into_response()
    }
}

fn into_response<T: Template>(t: &T) -> Response {
    match t.render() {
        Ok(body) => {
            let headers = [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static(T::MIME_TYPE),
            )];

            (headers, body).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
