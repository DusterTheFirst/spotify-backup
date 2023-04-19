use dioxus::prelude::*;

use crate::{pages::Page, router::authentication::CompleteUser};

pub async fn dashboard(auth: CompleteUser) -> Page<'static> {
    Page {
        title: rsx! { "Dashboard" },
        content: rsx! {
            h1 { "hello, i guess" }
            pre {
                "{auth:#?}"
            }
        },
    }
}
