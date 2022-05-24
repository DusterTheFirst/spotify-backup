import { html_response } from "./render";
import { h } from "preact";
import { SpotifyClient } from "./spotify";

export async function fetch_home(spotify: SpotifyClient | null) {
    if (spotify !== null) {
        const is_expired = Date.now() < spotify.oauth.expires_at;
        const expires_at = new Date(spotify.oauth.expires_at).toUTCString();

        const me = await spotify.me();

        return html_response(
            <section>
                <h1 style={{ color: "green" }}>user authenticated</h1>
                <div>as {me.display_name}</div>
                <div>
                    token
                    {is_expired ? (
                        <span style={{ color: "goldenrod" }}> expiring </span>
                    ) : (
                        <span style={{ color: "red" }}> expired </span>
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
            </section>
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
