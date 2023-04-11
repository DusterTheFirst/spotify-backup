use dioxus::prelude::*;

use super::Page;

pub async fn login() -> Page<'static> {
    Page {
        title: rsx! { "Login" },
        head: None,
        content: rsx! {
            h1 { "welcome" }

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
