use std::fmt::Debug;

use askama::Template;
use axohtml::{
    elements, html, text,
    types::{LinkType, Metadata},
};
use axum::{
    http::{self, StatusCode},
    response::{IntoResponse, Response},
};
use tracing::error;

pub mod error;

pub struct Page {
    pub title: String,
    pub content: Box<dyn elements::FlowContent<String>>,
}

impl Page {
    fn wrap(self) -> Box<elements::html<String>> {
        html! {
            <html lang="en">
                <head>
                    <title>{text!("Document | {}", self.title)}</title>

                    <meta charset="UTF-8"/>
                    <meta http-equiv="X-UA-Compatible" content="IE=edge"/>
                    <meta name=Metadata::Viewport content="width=device-width, initial-scale=1.0"/>

                    <link rel=LinkType::Icon href="/static/branding/logo-transparent@192.webp" type="image/webp"/>
                    <link rel="apple-touch-icon" type="image/png" sizes="192x192" href="/static/branding/logo@192.png"/>

                    <link rel="manifest" href="/static/manifest.json"/>
                </head>
                <body>
                    {self.content}
                </body>
            </html>
        }
    }
}

impl IntoResponse for Page {
    fn into_response(self) -> Response {
        let headers = [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("html"),
        )];

        (headers, self.wrap().to_string()).into_response()
    }
}

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
