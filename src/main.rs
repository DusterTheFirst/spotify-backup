use std::{env, future::Future, pin::Pin};

use argh::FromArgs;
use async_std::task;
use color_eyre::{
    eyre::{bail, eyre, Context, ContextCompat},
    Help,
};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    scopes, AuthCodeSpotify, Config, Credentials, OAuth,
};
use tracing::{debug, info, trace};
use tracing_subscriber::EnvFilter;

mod auth;
mod output;
mod web;

const REDIRECT_URL: &str = "http://localhost:8080";

/// Generate CSV files from spotify playlists
#[derive(FromArgs, Debug)]
struct Arguments {
    #[argh(subcommand)]
    command: Command,
}

#[derive(FromArgs, Debug)]
#[argh(subcommand)]
enum Command {
    GetToken(GetTokenArgs),
    Write(WriteArgs),
}

/// Get an authentication token for caching between runs
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "get-token")]
struct GetTokenArgs {}

/// Write the csv file
#[derive(FromArgs, Debug)]
#[argh(subcommand, name = "write")]
struct WriteArgs {}

fn main() -> color_eyre::Result<()> {
    // Setup error reporting
    color_eyre::install()?;

    // Setup logging
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(format!("{}=trace", env!("CARGO_CRATE_NAME")).parse()?),
        )
        .init();

    // Setup CI specific behavior
    let ci = env::var("CI").is_ok();

    if ci {
        debug!("Running in CI environment");
    } else {
        debug!("Not running in CI environment");
    }

    // Parse arguments
    let args: Arguments = argh::from_env();

    // Ensure valid options
    if matches!(args.command, Command::GetToken(_)) && ci {
        bail!("Cannot run get-token in a CI environment");
    }

    // Load the spotify credentials
    let creds = Credentials::from_env()
        .wrap_err("no rspotify credentials")
        .warning("make sure you are providing the required environment variables")
        .note("missing either RSPOTIFY_CLIENT_ID or RSPOTIFY_CLIENT_SECRET")?;

    // Setup the spotify client
    let spotify = AuthCodeSpotify::with_config(
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
    );

    // Drop into the async runtime after the initial setup
    task::block_on::<Pin<Box<dyn Future<Output = _>>>, _>(match args.command {
        Command::GetToken(args) => Box::pin(get_token(args, spotify)),
        Command::Write(args) => Box::pin(write(args, spotify)),
    })?;

    Ok(())
}

#[tracing::instrument(skip(spotify))]
async fn get_token(args: GetTokenArgs, mut spotify: AuthCodeSpotify) -> color_eyre::Result<()> {
    debug!("Updating credentials");
    auth::update_credentials(&mut spotify)
        .await
        .wrap_err("failed to update credentials")?;

    info!(
        cache_path = ?spotify.get_config().cache_path,
        "Credentials have been saved",
    );

    Ok(())
}

#[tracing::instrument(skip(spotify))]
async fn write(args: WriteArgs, mut spotify: AuthCodeSpotify) -> color_eyre::Result<()> {
    trace!("Reading token from token cache");
    let token = spotify
        .read_token_cache(true)
        .await
        .wrap_err("failed to read the token cache")
        .note("does the cache exist?")?;

    match token {
        Some(token) => *spotify.get_token().lock().await.unwrap() = Some(token),
        None => {
            return Err(eyre!("spotify authentication invalid").note(
                "you may need to update the scopes or refresh token manually with `get-token`",
            ))
        }
    }

    info!("Loading user's saved tracks");
    let liked_songs = spotify.current_user_saved_tracks(None);

    let filename = "./liked_songs.csv";
    let mut csv = csv::Writer::from_path(&filename)?;

    info!(?filename, "Writing saved tracks");
    output::write_all_records(&mut csv, liked_songs)
        .await
        .wrap_err("failed to write output data")
        .with_warning(|| format!("make sure the file {} is writeable", filename))?;

    Ok(())
}
