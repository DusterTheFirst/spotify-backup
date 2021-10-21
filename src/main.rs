use std::{convert::TryInto, str::FromStr};

use anyhow::Context;
use async_std::{
    stream::{Stream, StreamExt},
    task::block_on,
};
use rspotify::{
    clients::BaseClient,
    model::{SearchType, UserId},
    ClientCredsSpotify, Credentials,
};

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt().init();

    let creds = Credentials::from_env().context("No rspotify credentials")?;

    let mut spotify = ClientCredsSpotify::new(creds);

    block_on::<_, anyhow::Result<()>>(async {
        spotify.request_token().await?;

        let user_id = UserId::from_str("dusterthefirst")?;

        let mut playlists = spotify.user_playlists(&user_id);

        while let Some(playlist) = playlists.next().await.transpose()? {
            dbg!(playlist.name);
        }

        Ok(())
    })?;

    Ok(())
}
