use std::io::Write;

use async_std::prelude::StreamExt;
use color_eyre::eyre::Context;
use rspotify::{clients::pagination::Paginator, model::SavedTrack, ClientResult};

pub async fn write_all_records<'w, 'f, W: Write>(
    mut writer: csv::Writer<W>,
    mut song_list: Paginator<'f, ClientResult<SavedTrack>>,
) -> color_eyre::Result<()> {
    writer
        .write_record([
            "added at",
            "release date",
            "name",
            "album",
            "artist(s)",
            "id",
        ])
        .wrap_err("failed to write header")?;

    while let Some(song) = song_list.next().await {
        let SavedTrack { added_at, track } = song.wrap_err("failed to fetch song info")?;

        writer
            .write_record([
                added_at.to_rfc3339(),
                track.album.release_date.unwrap_or_default(),
                track.name,
                track.album.name,
                track
                    .artists
                    .iter()
                    .map(|artist| artist.name.as_str())
                    .collect::<Vec<&str>>()
                    .join("+"),
                track.id.to_string(),
            ])
            .wrap_err("failed to write record")?;
    }

    writer.flush().wrap_err("failed to flush the writer")?;

    Ok(())
}
