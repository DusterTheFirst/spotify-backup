use dioxus::prelude::*;

use crate::router::authentication::Account;

use super::Page;

pub async fn home(account: Option<Account>) -> Page<'static> {
    let user_info = account.map(|account| {
        rsx! {
            section {
                h2 { "Welcome back" }
                a { href: "/dashboard", "go to dashboard"}
            }
        }
    });

    Page {
        title: rsx! { "Home" },
        content: rsx! {
            nav {
                a {
                    href: "/dashboard"
                }

                section {
                    h1 { "Spotify Backup" }
                    p {
                        "This service is still in pre-release. "
                        "There are no guarantees of functionality or preservation "
                        "of user data until a release candidate is chosen"
                    }
                }

                user_info
            }
        },
    }
}
