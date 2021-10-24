use async_std::task;
use color_eyre::{
    eyre::{eyre, Context},
    Help,
};
use rspotify::{
    clients::{BaseClient, OAuthClient},
    AuthCodeSpotify,
};
use spotify_backup::{initialize, output};
use tracing::{info, trace};

fn main() -> color_eyre::Result<()> {
    let spotify = initialize(env!("CARGO_CRATE_NAME"))?;

    task::block_on(write(spotify))
}

#[tracing::instrument(skip(spotify))]
async fn write(mut spotify: AuthCodeSpotify) -> color_eyre::Result<()> {
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
