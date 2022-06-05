import { tracks_csv } from "./csv";
import { Environment, get_origin } from "./env";
import server_error from "./pages/error/500";
import SpotifyClient from "./spotify";

export async function dry_run(spotify: SpotifyClient | null, env: Environment) {
    if (spotify === null) {
        return Response.redirect(get_origin(env));
    }

    const tracks = await spotify.my_saved_tracks();

    if (!tracks.success) {
        return server_error("unable to fetch saved tracks", tracks.error);
    }

    const csv = tracks_csv(tracks.data);

    return new Response(csv, {
        headers: { "Content-Type": "text/csv" },
    });
}

export async function wet_run(spotify: SpotifyClient | null, env: Environment) {
    if (spotify === null) {
        return Response.redirect(get_origin(env));
    }

    const tracks = await spotify.my_saved_tracks();

    if (!tracks.success) {
        return server_error("unable to fetch saved tracks", tracks.error);
    }

    const csv = tracks_csv(tracks.data);

    const csv_utf8 = new TextEncoder().encode(csv);
    const config = {
        path: "liked_songs.csv",
        owner: "dusterthefirst",
        repo: "playlist",
    };

    const file_url = `https://api.github.com/repos/${config.owner}/${config.repo}/contents/${config.path}`;
    const branch = env.ENVIRONMENT === "dev" ? "test" : "main";
    const headers = {
        Authorization: `token ${env.GITHUB_ACCESS_TOKEN}`,
        Accept: "application/vnd.github.v3+json",
        "User-Agent": "spotify-backup cloudflare-workers",
    };

    const content_request = await fetch(file_url + `?ref=${branch}`, {
        method: "GET",
        headers,
    });

    if (!content_request.ok) {
        return server_error(`failed to fetch ${config.path}`, {
            status: content_request.status,
            statusText: content_request.statusText,
            response: await content_request.text(),
        });
    }

    type ContentResponse = { sha: string; content: string };
    const content_response = await content_request.json<
        ContentResponse | ContentResponse[]
    >();

    if (Array.isArray(content_response)) {
        return server_error(
            `${config.path} is a directory`,
            "received an array of objects, expected only one object"
        );
    }

    const csv_sha = await git_hash(csv_utf8);

    console.log(`old sha: ${content_response.sha}`);
    console.log(`new sha: ${csv_sha}`);

    if (content_response.sha == csv_sha) {
        console.log("hashes match, not creating commit");

        return Response.redirect(
            `https://github.com/DusterTheFirst/playlist/tree/${branch}`
        );
    }

    const update_request = await fetch(file_url, {
        method: "PUT",
        headers: {
            "Content-Type": "application/json",
            ...headers,
        },
        body: JSON.stringify({
            message: `Song update for ${new Date().toDateString()} @ ${new Date().getUTCHours()}:00 UTC\n\nThis commit was created automatically with https://github.com/DusterTheFirst/spotify-backup running on Cloudflare Workers`,
            content: btoa(
                csv_utf8.reduce(
                    (string, char_code, index, array) =>
                        string + String.fromCharCode(char_code),
                    ""
                )
            ),
            sha: content_response.sha,
            branch: env.ENVIRONMENT === "dev" ? "test" : undefined,
            committer: {
                email: "41898282+github-actions[bot]@users.noreply.github.com",
                name: "github-actions[bot]",
            },
        }),
    });

    if (!update_request.ok) {
        return server_error(`failed to update ${config.path}`, {
            response: await update_request.text(),
            status: update_request.status,
            statusText: update_request.statusText,
        });
    }

    const update_response = await update_request.json<{
        commit: { sha: string };
        content: ContentResponse;
    }>();

    return Response.redirect(
        `https://github.com/DusterTheFirst/playlist/commit/${update_response.commit.sha}`
    );
}

async function git_hash(utf8: Uint8Array) {
    const hash_header = new TextEncoder().encode(
        "blob " + utf8.length.toString() + "\0"
    );

    const stream = new Uint8Array(hash_header.length + utf8.length);
    stream.set(hash_header);
    stream.set(utf8, hash_header.length);

    return new Uint8Array(await crypto.subtle.digest("sha-1", stream)).reduce(
        (string, byte) => string + byte.toString(16).padStart(2, "0"),
        ""
    );
}
