import { tracks_csv } from "./csv";
import { Environment } from "./env";
import SpotifyClient from "./spotify";

async function get_csv(spotify: SpotifyClient | null) {
    if (spotify === null) {
        return new Response("spotify not authorized", {
            status: 401,
        });
    } else {
        const tracks = await spotify.my_saved_tracks();

        if (tracks === null) {
            return new Response("unable to get saved tracks", {
                status: 500,
            });
        } else {
            return tracks_csv(tracks);
        }
    }
}

export async function dry_run(spotify: SpotifyClient | null) {
    const csv = await get_csv(spotify);

    if (typeof csv === "string") {
        return new Response(csv, {
            headers: { "Content-Type": "text/csv" },
        });
    } else {
        return csv;
    }
}

export async function wet_run(spotify: SpotifyClient | null, env: Environment) {
    const csv = await get_csv(spotify);

    if (typeof csv !== "string") {
        return csv;
    }

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
        const response = await content_request.text();
        console.log(
            `failed to fetch ${config.path}. ${content_request.status}: ${content_request.statusText}; ${response}`
        );

        return new Response(`failed to fetch ${config.path}`, { status: 500 });
    }

    type ContentResponse = { sha: string; content: string };
    const content_response = await content_request.json<
        ContentResponse | ContentResponse[]
    >();

    if (Array.isArray(content_response)) {
        console.log(`${config.path} is a directory`);
        return new Response(`${config.path} is a directory`, { status: 500 });
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
            message: `Song update for ${new Date().toDateString()}`,
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
        const response = await update_request.text();
        console.log(
            `failed to update ${config.path}. ${content_request.status}: ${content_request.statusText}; ${response}`
        );

        return new Response(`failed to update ${config.path}`, { status: 500 });
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
