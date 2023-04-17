use dioxus::prelude::*;

use crate::{pages::Page, router::authentication::Account};

pub async fn dashboard(auth: Account) -> Page<'static> {
    Page {
        title: rsx! { "Dashboard" },
        content: rsx! {
            code { "hello, i guess" }
            pre {
                "{auth:?}"
            }
        },
    }
}
