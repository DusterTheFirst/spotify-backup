use async_std::task;
use color_eyre::eyre::{Context, ContextCompat};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    AuthCodeSpotify,
};
use spotify_backup::{initialize, web::OneOffWebServer};
use tracing::{debug, info, trace};

fn main() -> color_eyre::Result<()> {
    dotenv::dotenv().ok();

    let spotify = initialize(env!("CARGO_CRATE_NAME"))?;

    task::block_on(get_token(spotify))
}

#[tracing::instrument(skip(spotify))]
async fn get_token(mut spotify: AuthCodeSpotify) -> color_eyre::Result<()> {
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

    Ok(())
}
