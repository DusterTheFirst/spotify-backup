import octokit from "octokit";
import spotify from "spotify-api.js";

interface Environment {
    readonly SPOTIFY_BACKUP_KV: KVNamespace;
    readonly SPOTIFY_CLIENT_ID: string;
    readonly SPOTIFY_CLIENT_SECRET: string;
}
const SPOTIFY_KV_TOKEN = "spotify-token";
interface SpotifyKV {
    readonly token: string;
    readonly refresh_token: string;
}

function html_response(body: string, head: string = "") {
    return new Response(
        `<!DOCTYPE html><head>${head}</head><body>${body} </body>
        `,
        {
            headers: {
                "content-type": "text/html;charset=UTF-8",
            },
        }
    );
}

async function fetch_home(spotify_token: SpotifyKV | null) {
    if (spotify_token !== null) {
        return html_response(
            `user already authenticated<br/><a href="/start-auth">Re-authenticate</a>`
        );
    }

    return html_response(
        `user not authenticated<br/><a href="/start-auth">Authenticate</a>`
    );
}

async function authenticate_spotify(
    client_id: string,
    client_secret: string
): Promise<Response> {
    return Response.redirect("https://com", 307);
}

export default {
    async fetch(
        request: Request,
        {
            SPOTIFY_BACKUP_KV,
            SPOTIFY_CLIENT_ID,
            SPOTIFY_CLIENT_SECRET,
        }: Environment
    ): Promise<Response> {
        let spotify_token = await SPOTIFY_BACKUP_KV.get<SpotifyKV>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        let url = new URL(request.url);

        if (request.method === "GET") {
            switch (url.pathname) {
                case "/":
                    return await fetch_home(spotify_token);
                case "/start-auth":
                    return await authenticate_spotify(
                        SPOTIFY_CLIENT_ID,
                        SPOTIFY_CLIENT_SECRET
                    );
                default:
                    return new Response(`route not found`, { status: 404 });
            }
        } else {
            return new Response(`unexpected request method`, { status: 400 });
        }
    },
    async scheduled(
        event: ScheduledEvent,
        {
            SPOTIFY_BACKUP_KV,
            SPOTIFY_CLIENT_ID,
            SPOTIFY_CLIENT_SECRET,
        }: Environment
    ) {
        let spotify_token = await SPOTIFY_BACKUP_KV.get<SpotifyKV>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        if (spotify_token === null) {
            console.warn("No spotify token :(");
            return;
        }

        const client = new spotify.Client({
            token: {
                clientID: SPOTIFY_CLIENT_ID,
                clientSecret: SPOTIFY_CLIENT_SECRET,
                token: spotify_token.token,
                refreshToken: spotify_token.refresh_token,
            },
            refreshToken: true,
            retryOnRateLimit: true,
            userAuthorizedToken: true,
            cacheSettings: true,
        });

        console.log(event);
    },
};
