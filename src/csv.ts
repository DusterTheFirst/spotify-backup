export function tracks_csv(tracks: SpotifyApi.SavedTrackObject[]): string {
    let csv = csv_row([
        "added at",
        "release date",
        "name",
        "album",
        "artist(s)",
        "id",
    ]);

    for (const { added_at, track } of tracks) {
        csv += csv_row([
            to_rfc3339(new Date(added_at)),
            (track.album as SpotifyApi.AlbumObjectFull).release_date,
            track.name,
            track.album.name,
            track.artists.map((artist) => artist.name).join("+"),
            `spotify:track:${track.id}`,
        ]);
    }

    return csv;
}

export function to_rfc3339(date: Date): string {
    const year = date.getUTCFullYear().toString().padStart(4, "0");
    const month = (date.getUTCMonth() + 1).toString().padStart(2, "0");
    const day = date.getUTCDate().toString().padStart(2, "0");

    const hours = date.getUTCHours().toString().padStart(2, "0");
    const minutes = date.getUTCMinutes().toString().padStart(2, "0");
    const seconds = date.getUTCSeconds().toString().padStart(2, "0");

    return `${year}-${month}-${day}T${hours}:${minutes}:${seconds}+00:00`;
}

// Shitty CSV serializer
export function csv_row(row: string[]): string {
    let csv = "";

    for (const item of row) {
        if (csv.length != 0) {
            csv += ",";
        }

        if (/[,"]/g.test(item)) {
            csv += `"${item.replaceAll('"', '""')}"`;
        } else {
            csv += item;
        }
    }

    return csv + "\n";
}
