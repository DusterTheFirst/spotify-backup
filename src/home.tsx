import { html_response } from "./render";
import { h, Fragment } from "preact";
import SpotifyClient from "./spotify";
import { tracks_csv } from "./csv";

export async function fetch_home(spotify: SpotifyClient | null) {
    if (spotify !== null) {
        const is_expired = spotify.oauth.expired();
        const expires_at = new Date(
            spotify.oauth.storage.expires_at
        ).toUTCString();

        const me = await spotify.me();
        const saved = await spotify.my_saved_tracks();

        return html_response(
            <>
                <h1 style={{ color: "green" }}>user authenticated</h1>
                <div>as {me?.display_name ?? "ERROR"}</div>
                <div>
                    token
                    {is_expired ? (
                        <span style={{ color: "red" }}> expired </span>
                    ) : (
                        <span style={{ color: "goldenrod" }}> expiring </span>
                    )}
                    at {expires_at}
                </div>
                <div>it is currently {new Date().toUTCString()}</div>
                <ul>
                    <li>
                        <a href="/auth">re-authenticate</a>
                    </li>
                    <li>
                        <a href="/de-auth">de-authenticate</a>
                    </li>
                </ul>
                <details>
                    <summary>user</summary>
                    <code>
                        <pre>{JSON.stringify(me, undefined, 4)}</pre>
                    </code>
                </details>
                <details>
                    <summary>oauth</summary>
                    <code>
                        <pre>{JSON.stringify(spotify.oauth, undefined, 4)}</pre>
                    </code>
                </details>
                <details>
                    <summary>{saved?.length ?? 0} saved tracks</summary>
                    <code>
                        <pre>
                            {saved === null ? "none" : tracks_csv(saved)}
                        </pre>
                    </code>
                </details>
            </>
        );
    }

    return html_response(
        <section>
            <h1 style={{ color: "red" }}>user not authenticated</h1>
            <ul>
                <li>
                    <a href="/auth">authenticate</a>
                </li>
            </ul>
        </section>
    );
}
