use axum::response::Redirect;
use dioxus::prelude::*;

use crate::{pages::Page, router::authentication::Authentication};

pub async fn dashboard(auth: Authentication) -> Result<Page<'static>, Redirect> {
    Ok(Page {
        title: rsx! { "Dashboard" },
        head: None,
        content: rsx! {
            code { "hello, i guess" }
            pre {
                "{auth:?}"
            }
        },
    })
}
