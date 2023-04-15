use axum::response::Redirect;
use dioxus::prelude::*;

use crate::{pages::Page, router::authentication::Account};

pub async fn dashboard(auth: Account) -> Result<Page<'static>, Redirect> {
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
