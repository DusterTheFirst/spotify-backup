use std::{cmp::Ordering, io::Write};

use async_std::prelude::StreamExt;
use color_eyre::eyre::Context;
use rspotify::{clients::pagination::Paginator, model::SavedTrack, ClientResult};

pub async fn write_all_records<'w, 'f, W: Write>(
    mut writer: csv::Writer<W>,
    mut song_list: Paginator<'f, ClientResult<SavedTrack>>,
) -> color_eyre::Result<()> {
    // Intermediate buffer to allow for sorting
    let mut songs: Vec<SavedTrack> = Vec::with_capacity(song_list.size_hint().0);

    while let Some(song) = song_list.next().await {
        songs.push(song.wrap_err("failed to fetch song info")?);
    }

    // Make absolutely sure that the items are sorted in a deterministic manner
    songs.sort_unstable_by(|a, b| match b.added_at.cmp(&a.added_at) {
        Ordering::Equal => match a.track.name.cmp(&b.track.name) {
            Ordering::Equal => a.track.album.name.cmp(&b.track.album.name),
            order => order,
        },
        order => order,
    });

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

    for SavedTrack { added_at, track } in songs {
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
