#![forbid(unsafe_code)]
#![deny(clippy::unwrap_in_result, clippy::unwrap_used)]
#![allow(clippy::new_without_default)]

use std::{borrow::Cow, env, net::SocketAddr, path::PathBuf};

use axum::http::{uri::Authority, Uri};
use serde::Deserialize;
use tracing_subscriber::{prelude::*, EnvFilter};

mod database;
mod pages;
mod router;

pub struct HttpEnvironment {
    bind: SocketAddr,
    static_dir: PathBuf,
    domain: Authority,
}

impl HttpEnvironment {
    fn from_env() -> Self {
        HttpEnvironment {
            bind: env::var("BIND")
                .expect("$BIND should be set")
                .parse()
                .expect("$BIND should be a valid SocketAddr"),
            static_dir: env::var_os("STATIC_DIR")
                .expect("$STATIC_DIR should be set")
                .into(),
            domain: env::var("DOMAIN")
                .expect("$DOMAIN should be set")
                .parse::<Authority>()
                .expect("$DOMAIN should be a valid URI authority"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SpotifyEnvironment {
    credentials: rspotify::Credentials,
    redirect_uri: Uri,
}

impl SpotifyEnvironment {
    pub fn from_env() -> Self {
        SpotifyEnvironment {
            credentials: rspotify::Credentials {
                id: env::var("SPOTIFY_CLIENT_ID").expect("$SPOTIFY_CLIENT_ID should be set"),
                secret: Some(
                    env::var("SPOTIFY_CLIENT_SECRET")
                        .expect("$SPOTIFY_CLIENT_SECRET should be set"),
                ),
            },
            redirect_uri: env::var("SPOTIFY_REDIRECT_URI")
                .expect("$SPOTIFY_REDIRECT_URI should be set")
                .parse()
                .expect("$SPOTIFY_REDIRECT_URI should be a valid URI"),
        }
    }
}

#[derive(Deserialize)]
pub struct GithubEnvironment {}

fn main() -> Result<(), color_eyre::Report> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .with(sentry::integrations::tracing::layer())
        .init();

    let _guard = sentry::init(sentry::ClientOptions {
        dsn: env::var("SENTRY_DSN")
            .ok()
            .map(|dsn| dsn.parse().expect("SENTRY_DSN should be a valid DSN")),
        // TODO: setup release tracking
        release: Some(git_version::git_version!(args = ["--always", "--abbrev=40"]).into()), // sentry::release_name!(), // TODO: use git hash?
        sample_rate: 1.0,
        traces_sample_rate: 0.0, // TODO: make not 0, but also not spammy
        enable_profiling: true,
        profiles_sample_rate: 1.0,
        attach_stacktrace: true,
        send_default_pii: true,
        server_name: env::var("FLY_REGION").map(Cow::from).ok(),
        in_app_include: vec!["spotify_backup"],
        // in_app_exclude: todo!(),
        auto_session_tracking: true,
        session_mode: sentry::SessionMode::Request,
        trim_backtraces: true,
        ..Default::default()
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builder should succeed")
        .block_on(router::router(
            HttpEnvironment::from_env(),
            SpotifyEnvironment::from_env(),
            GithubEnvironment {},
        ))
}
