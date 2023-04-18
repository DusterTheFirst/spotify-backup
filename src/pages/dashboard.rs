use dioxus::prelude::*;

use crate::{pages::Page, router::authentication::User};

pub async fn dashboard(auth: User) -> Page<'static> {
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
