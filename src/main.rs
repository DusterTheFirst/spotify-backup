use std::path::Path;

use argh::FromArgs;
use async_std::task;
use color_eyre::{
    eyre::{Context, ContextCompat},
    Help,
};
use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature, Status};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    scopes, AuthCodeSpotify, Config, Credentials, OAuth,
};
use temp_dir::TempDir;
use tracing::{debug, info, metadata::LevelFilter, trace};
use tracing_subscriber::EnvFilter;

use crate::web::OneOffWebServer;

mod output;
mod web;

pub const REDIRECT_URL: &str = "http://localhost:8080";

#[derive(Debug, FromArgs)]
/// Clone a repository of spotify songs and update the csv files
struct Arguments {
    /// the repository to clone
    #[argh(option, short = 'r', long = "repo")]
    repo: String,
    /// the filename of the csv file
    #[argh(
        option,
        short = 'f',
        long = "filename",
        default = "\"liked_songs.csv\".into()"
    )]
    filename: String, // TODO: more than just liked songs
}

fn main() -> color_eyre::Result<()> {
    let args: Arguments = argh::from_env();

    // Setup error reporting
    color_eyre::install()?;

    // Load dot-file
    dotenv::dotenv().ok();

    // Setup logging
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive(LevelFilter::INFO.into())
                .add_directive("rspotify=warn".parse()?)
                .add_directive(format!("{}=trace", env!("CARGO_CRATE_NAME")).parse()?),
        )
        .init();

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

    task::block_on(start(spotify, args))
}

async fn start(mut spotify: AuthCodeSpotify, args: Arguments) -> color_eyre::Result<()> {
    trace!("Reading token from token cache");

    match spotify.read_token_cache(true).await {
        Ok(Some(token)) => *spotify.get_token().lock().await.unwrap() = Some(token),
        _ => {
            debug!("Updating credentials");

            webbrowser::open(&spotify.get_authorize_url(false)?)?;
            trace!("Opened web browser to auth URL");

            let auth_code = OneOffWebServer::new()
                .wait_for_request()
                .await
                .wrap_err("failed to get user auth")?;

            let auth_code = spotify
                .parse_response_code(&auth_code)
                .wrap_err("failed to parse auth code")?;

            trace!("Requesting new token");
            spotify.request_token(&auth_code).await?;

            info!(
                cache_path = ?spotify.get_config().cache_path,
                "Credentials have been saved",
            );
        }
    }

    info!("Loading user's saved tracks");
    let liked_songs = spotify.current_user_saved_tracks(None);

    let temp_dir = TempDir::new().wrap_err("failed to create temporary directory")?;
    debug!(dir = ?temp_dir.path(), "Using temporary directory");

    let repo = Repository::clone(&args.repo, temp_dir.path())
        .wrap_err("failed to clone repository")
        .with_note(|| format!("make sure you have permission to clone {}", args.repo))?;

    let csv_file = &args.filename;
    let mut csv = csv::Writer::from_path(temp_dir.child(csv_file))?;

    info!(?csv_file, "Writing saved tracks");
    output::write_all_records(&mut csv, liked_songs)
        .await
        .wrap_err("failed to write output data")
        .with_warning(|| format!("make sure the file {} is writeable", &args.filename))?;

    info!(?csv_file, "Done writing saved tracks");

    let file_status = repo
        .status_file(Path::new(&csv_file))
        .wrap_err("failed to get file status")?;

    if file_status != Status::CURRENT {
        trace!(?csv_file, "File has changed... creating new commit");

        let config = git2::Config::open_default().wrap_err("failed to open global git config")?;

        let signature = &Signature::now(
            &config
                .get_string("user.name")
                .wrap_err("failed to load user.name from git config")
                .note("is your git configured correctly")?,
            &config
                .get_string("user.email")
                .wrap_err("failed to load user.name from git config")
                .note("is your git configured correctly")?,
        )
        .wrap_err("failed to create git signature")?;

        let head = repo.head().wrap_err("failed to get HEAD")?;

        let head_tree = head
            .peel_to_tree()
            .wrap_err("failed to get tree form HEAD")?;

        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "New songs",
            &head_tree,
            &[&repo
                .find_commit(
                    head.resolve()
                        .wrap_err("failed to resolve the HEAD reference")?
                        .target()
                        .expect("target should have an Oid"),
                )
                .wrap_err("failed to find the HEAD commit")?],
        )
        .wrap_err("failed to commit changes")?;

        trace!("Committed");

        let mut remote = repo
            .find_remote("origin")
            .wrap_err("failed to find remote `origin`")?;

        let mut push_options = {
            let remote = remote.clone();

            let mut callbacks = RemoteCallbacks::new();
            callbacks.credentials(move |_, _, _| {
                Cred::credential_helper(
                    &config,
                    remote.url().expect("remote url should be valid utf-8"),
                    None,
                )
            });

            let mut options = PushOptions::new();
            options.remote_callbacks(callbacks);

            options
        };

        let refspecs = remote.push_refspecs().wrap_err("failed to get refspecs")?;

        remote
            .push(
                &refspecs.iter().filter_map(|x| x).collect::<Vec<_>>(),
                Some(&mut push_options),
            )
            .wrap_err("Failed to push")?;

        trace!("Pushed");
    } else {
        info!("File has not changed, nothing to commit");
    }

    Ok(())
}
