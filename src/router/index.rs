use dioxus::prelude::*;

use crate::pages::Page;

pub async fn index() -> Page<'static> {
    Page {
        title: rsx! { "Home" },
        head: None,
        content: rsx! {
            menu {
                li {
                    a { href: "/login/spotify",
                        "log in with spotify"
                    }
                }
                li {
                    a { href: "/login/github",
                        "log in with github"
                    }
                }
                li {
                    a { href: "/login",
                        "i'm new, help me get set up"
                    }
                }
            }
        },
    }
}
