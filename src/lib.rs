use color_eyre::{eyre::ContextCompat, Help};
use rspotify::{scopes, AuthCodeSpotify, Config, Credentials, OAuth};
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

const REDIRECT_URL: &str = "http://localhost:8080";

#[cfg(feature = "setup")]
pub mod web;

#[cfg(feature = "bootstrap")]
pub mod output;

pub fn initialize(crate_name: &str) -> color_eyre::Result<AuthCodeSpotify> {
    // Setup error reporting
    color_eyre::install()?;

    // Setup logging
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(LevelFilter::INFO.into())
                .add_directive("rspotify=warn".parse()?)
                .add_directive(format!("{}=trace", env!("CARGO_CRATE_NAME")).parse()?)
                .add_directive(format!("{}=trace", crate_name).parse()?),
        )
        .init();

    // Load the spotify credentials
    let creds = Credentials::from_env()
        .wrap_err("no rspotify credentials")
        .warning("make sure you are providing the required environment variables")
        .note("missing either RSPOTIFY_CLIENT_ID or RSPOTIFY_CLIENT_SECRET")?;

    // Setup the spotify client
    Ok(AuthCodeSpotify::with_config(
        creds,
        OAuth {
            scopes: scopes!("user-library-read"),
            redirect_uri: REDIRECT_URL.into(),
            ..Default::default()
        },
        Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        },
    ))
}
