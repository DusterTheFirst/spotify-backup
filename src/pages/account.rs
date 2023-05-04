use dioxus::prelude::*;
use rspotify::prelude::Id;
use tokio::try_join;

use crate::router::authentication::IncompleteUser;

use super::{InternalServerError, Page};

pub async fn account(current_user: IncompleteUser) -> Result<Page<'static>, InternalServerError> {
    let (spotify_user, github_user) = try_join!(
        current_user.account.spotify_user(),
        current_user.account.github_user()
    )?;
    let user_complete = current_user.is_complete();

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
                li {
                    if let Some(user) = spotify_user {
                        let spotify_name = user.display_name.unwrap_or_else(|| user.id.id().to_string());

                        rsx! {
                            "spotify already authenticated as {spotify_name}"
                            a { href: "/login/spotify",
                                "change spotify account"
                            }
                        }
                    } else{
                        rsx! {
                            a { href: "/login/spotify",
                                "log in with spotify"
                            }
                        }
                    }
                }
                li {
                    if let Some(user) = github_user {
                        let github_name = user.login;

                        rsx! {
                            "github already authenticated as {github_name}"
                            a { href: "/login/github",
                                "change github account"
                            }
                        }
                    } else { rsx! {
                            a { href: "/login/github",
                                "log in with github"
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
