use axum::{
    http,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;

mod dashboard;
mod error;
mod login;

pub use {
    dashboard::dashboard,
    error::{dyn_error, not_found, panic_error, EyreReport},
    login::login,
};

pub struct Page<'e> {
    pub title: LazyNodes<'e, 'e>,
    pub head: Option<LazyNodes<'e, 'e>>,
    pub content: LazyNodes<'e, 'e>,
}

impl<'e> Page<'e> {
    fn wrap(self) -> LazyNodes<'e, 'e> {
        rsx! {
            head {
                meta { charset: "utf-8"}
                meta {
                    http_equiv: "X-UA-Compatible",
                    content: "IE=edge"
                }
                meta {
                    name: "viewport",
                    content: "width=device-width, initial-scale=1.0"
                }

                link {
                    rel: "icon",
                    href: "/static/branding/logo-transparent@192.webp",
                    r#type: "image/webp",
                }
                link {
                    rel: "apple-touch-icon",
                    href: "/static/branding/logo@192.png",
                    r#type: "image/png",
                    sizes: "192x192"
                }

                link { rel: "manifest", href: "/static/manifest.json"}

                title { self.title, " - Spotify Backup" }

                self.head
            }
            body {
                self.content
            }
        }

        // <!DOCTYPE html>
        // <html lang="en">
        // </html>
    }
}

impl<'e> IntoResponse for Page<'e> {
    fn into_response(self) -> Response {
        let headers = [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/html; charset=UTF-8"),
        )];

        let page = dioxus_ssr::render_lazy(self.wrap());

        let html = format!("<!DOCTYPE html><html lang=\"en\">{}</html>", page);

        (headers, html).into_response()
    }
}
