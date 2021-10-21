use std::{env, str::FromStr};

use anyhow::Context;
use async_std::{stream::StreamExt, task::block_on};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    model::{SimplifiedPlaylist, UserId},
    scopes, AuthCodeSpotify, ClientCredsSpotify, Credentials, OAuth,
};

fn main() -> anyhow::Result<()> {
    let ci = env::var("CI").is_ok();

    tracing_subscriber::fmt().init();

    let creds = Credentials::from_env().context("No rspotify credentials")?;
    let oauth = OAuth {
        scopes: scopes!("playlist-read-private"),
        redirect_uri: "http://localhost:8080/callback".into(),
        ..Default::default()
    };

    let mut spotify = AuthCodeSpotify::new(creds, oauth);

    block_on::<_, anyhow::Result<()>>(async {
        spotify
            .prompt_for_token(&spotify.get_authorize_url(false)?)
            .await?;

        let user_id = UserId::from_str("dusterthefirst")?;

        let mut playlists = spotify.user_playlists(&user_id);

        while let Some(playlist) = playlists.next().await {
            let playlist: SimplifiedPlaylist = playlist?;

            dbg!(playlist.name);
        }

        Ok(())
    })?;

    Ok(())
}
