import { h, Fragment } from "preact";
import html_response from "../render/response";
import SpotifyClient from "../spotify";
import server_error from "./error/500";

export default async function home(spotify: SpotifyClient | null) {
    if (spotify === null) {
        return html_response(<NotAuthenticated />);
    }

    const is_expired = spotify.oauth.expired();

    if (is_expired) {
        return html_response(<TokenExpired />);
    }

    const me = await spotify.me();

    if (!me.success) {
        return server_error("failed to fetch spotify user's details", me.error);
    }

    return html_response(
        <UserInformation me={me.data} oauth={spotify.oauth} />
    );
}

function NotAuthenticated() {
    return (
        <main>
            <section>
                <h1 style={{ color: "red" }}>user not authenticated</h1>
                <ul>
                    <li>
                        <a href="/auth">authenticate</a>
                    </li>
                </ul>
            </section>

            <footer>
                <a href="javascript:window.location.reload()">refresh</a>
            </footer>
        </main>
    );
}

function TokenExpired() {
    return (
        <main>
            <section>
                <h1 style={{ color: "yellow" }}>user authenticated</h1>
                <p>but the token I have is expired</p>
                <hr />
                <p>
                    this could happen for a few reasons
                    <ol>
                        <li>the refresh token has been revoked by the user</li>
                        <li>the refresh token has expired somehow</li>
                        <li>there was a problem refreshing the token</li>
                        <li>
                            javascript sucks and something else has failed
                            silently
                        </li>
                    </ol>
                    to find out which one of these it was, you'll have to set up
                    logging for this :) good luck
                </p>
                <p>
                    or you could just ignore this happened and just
                    <ul>
                        <li>
                            <a href="/auth">re-authenticate</a>
                        </li>
                    </ul>
                </p>
            </section>
        </main>
    );
}

function UserInformation({
    me,
    oauth,
}: {
    me: SpotifyApi.CurrentUsersProfileResponse;
    oauth: SpotifyClient["oauth"];
}) {
    const expires_at = new Date(oauth.storage.expires_at).toUTCString();

    return (
        <main>
            <h1 style={{ color: "green" }}>user authenticated</h1>
            <div>authenticated as {me.display_name ?? me.id}</div>
            <div>token expires at {expires_at}</div>
            <div>it is currently {new Date().toUTCString()}</div>
            <ul>
                <li>
                    <a href="/auth">re-authenticate</a>
                </li>
                <li>
                    <a href="/de-auth">de-authenticate</a>
                </li>
            </ul>
            <ul>
                <li>
                    <a href="/dry-run">download csv</a>
                </li>
                <li>
                    <a href="/wet-run">create commit now</a>
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
                    <pre>{JSON.stringify(oauth, undefined, 4)}</pre>
                </code>
            </details>
        </main>
    );
}
