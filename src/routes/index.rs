use axohtml::html;

use crate::pages::Page;

pub async fn index() -> Page {
    Page {
        title: "Home".into(),
        content: html! { "hello" },
    }
}
