use axum::{
    http,
    response::{IntoResponse, Response},
};
use dioxus::prelude::*;

use crate::router::middleware::server_information::SERVER_INFO;

mod account;
mod dashboard;
mod error;
mod home;

pub use {
    account::account,
    dashboard::dashboard,
    error::{not_found, panic_error, InternalServerError, ClientError},
    home::home,
};

pub struct Page<'e> {
    pub title: LazyNodes<'e, 'e>,
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
                link { rel: "stylesheet", href: "/static/styles.css" }

                link { rel: "manifest", href: "/static/manifest.json" }

                if cfg!(feature = "live_js") {
                    rsx! {
                        script { src: "https://livejs.com/live.js#css,js" }
                    }
                }

                title { self.title, " - Spotify Backup" }
            }
            body {
                self.content
                footer {
                    SERVER_INFO.name
                    " version "
                    SERVER_INFO.version
                    " commit "
                    a { href: SERVER_INFO.source,
                        target: "_blank",
                        SERVER_INFO.commit
                    }
                    " environment "
                    SERVER_INFO.environment
                }
            }
        }
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
