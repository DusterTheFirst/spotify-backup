use axum::response::{IntoResponse, Response};
use dioxus::prelude::*;

use crate::router::authentication::IncompleteUser;

use super::Page;

pub async fn account(current_user: IncompleteUser) -> Response {
    let spotify_login = current_user.account.spotify.as_ref();
    let github_login = current_user.account.github.as_ref();
    let user_complete = current_user.is_complete();

    // TODO: make this a general account management page
    Page {
        title: rsx! { "Account" },
        content: rsx! {
            h1 { "Welcome" }
            if user_complete {
                rsx! {
                    "manage your account"
                }
            } else {
                rsx! {
                    "finish setting up your account"
                }
            }

            menu {
                li {
                    if let Some(login) = spotify_login {
                        rsx! {
                            "spotify already authenticated as {login}"
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
                    if let Some(login) = github_login {
                        rsx! {
                            "github already authenticated as {login}"
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
            }
        },
    }
    .into_response()
}
