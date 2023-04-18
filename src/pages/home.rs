use dioxus::prelude::*;

use crate::router::authentication::User;

use super::Page;

pub async fn home(account: Option<User>) -> Page<'static> {
    let navigation = match account {
        Some(user) => rsx! {
            section {
                h2 { "Welcome back" }
                a { href: "/logout", "log out" }
                a { href: "/dashboard", "go to dashboard" }
                pre { "{user.account:#?}" }
            }
        },
        None => rsx! {
            section {
                h2 { "Welcome" }
                a { href: "/login", "Login" }
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
