use dioxus::prelude::rsx;

use crate::pages::Page;

pub async fn index() -> Page<'static> {
    Page {
        title: rsx! { "Home" },
        head: None,
        content: rsx! { "hello" },
    }
}
