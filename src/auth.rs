use color_eyre::eyre::{Context, ContextCompat};
use rspotify::{clients::OAuthClient, AuthCodeSpotify};
use tracing::{debug, trace};

use crate::web::OneOffWebServer;

#[tracing::instrument(skip(spotify), err)]
pub async fn update_credentials(spotify: &mut AuthCodeSpotify) -> color_eyre::Result<()> {
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

    debug!("Success");

    Ok(())
}
