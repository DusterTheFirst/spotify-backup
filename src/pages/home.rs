use dioxus::prelude::*;

use crate::router::authentication::IncompleteUser;

use super::Page;

pub async fn home(account: Option<IncompleteUser>) -> Page<'static> {
    let navigation = match account {
        Some(user) if user.is_complete() => rsx! {
            // TODO: if incomplete, tell them to finish setting up account
            section {
                h2 { "Welcome back" }
                ul {
                    li { a { href: "/logout", "log out" } }
                    li { a { href: "/dashboard", "go to dashboard" } }
                }
                pre { "{user.account:#?}" }
            }
        },
        Some(user) => rsx! {
            section {
                h2 { "Welcome back" }
                ul {
                    li { a { href: "/logout", "log out" } }
                    li { a { href: "/account", "finish setting up your account" } }
                }
                pre { "{user.account:#?}" }
            }
        },
        None => rsx! {
            section {
                h2 { "Welcome" }
                ul {
                    li { a { href: "/login/github", "Login With Github" } }
                    li { a { href: "/login/spotify", "Login With Spotify" } }
                }
            }
        },
    };

    Page {
        title: rsx! { "Home" },
        content: rsx! {
            nav {
                section {
                    h1 { "Spotify Backup" }
                    p {
                        "This service is still in pre-release. "
                        "There are no guarantees of functionality or preservation "
                        "of user data until a release candidate is chosen"
                    }
                }

                navigation
            }
        },
    }
}
