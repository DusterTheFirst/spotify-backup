use dioxus::prelude::*;
use rspotify::prelude::Id;
use tokio::try_join;

use crate::router::authentication::User;

use super::{InternalServerError, Page};

pub async fn account(current_user: User) -> Result<Page<'static>, InternalServerError> {
    let (spotify_user, github_user) = try_join!(
        current_user.account.spotify_user(),
        current_user.account.github_user()
    )?;
    let user_complete = github_user.is_some();

    let spotify_name = spotify_user
        .display_name
        .unwrap_or_else(|| spotify_user.id.id().to_string());

    Ok(Page {
        title: rsx! { "Account" },
        content: rsx! {
            h1 { "Account" }
            if user_complete {
                rsx! {
                    p { "manage your account" }
                }
            } else {
                rsx! {
                    p { "you must finish setting up your account, before you can use the service" }
                }
            }

            menu {
                if user_complete {
                    rsx! {
                        li {
                            a { href: "/dashboard",
                                "dashboard"
                            }
                        }
                        hr {}
                    }
                }
                h2 { "Music source" }
                li {
                    "spotify authenticated as {spotify_name}"
                    a { href: "/login/spotify",
                        "change spotify account"
                    }
                }
                h2 { "Backup destination" }
                li {
                    if let Some(user) = github_user {
                        let github_name = user.login;

                        rsx! {
                            "github authenticated as {github_name}"
                            a { href: "/login/github",
                                "change github account"
                            }
                            a { href: "/logout/github",
                                "remove github account"
                            }
                        }
                    } else { rsx! {
                            a { href: "/login/github",
                                "add github account"
                            }
                        }
                    }
                }
                hr {}
                li {
                    a { href: "/logout",
                        "log out"
                    }
                }
                li {
                    a { href: "/logout/delete",
                        "delete account"
                    }
                }
            }
        },
    })
}
