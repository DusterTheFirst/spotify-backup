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

const SPOTIFY_API = "https://api.spotify.com/";
const SPOTIFY_ACCOUNTS = "https://accounts.spotify.com/";

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

async function fetch_home(spotify_token: SpotifyKV | null, auth_url: URL) {
    if (spotify_token !== null) {
        return html_response(
            `user already authenticated<br/><a href="${auth_url}">Re-authenticate</a>`
        );
    }

    return html_response(
        `user not authenticated<br/><a href="${auth_url}">Authenticate</a>`
    );
}

function create_authentication_url(env: Environment, origin: string): URL {
    let auth_url = new URL(SPOTIFY_ACCOUNTS);
    auth_url.pathname = "/authorize";
    auth_url.searchParams.append("client_id", env.SPOTIFY_CLIENT_ID);
    auth_url.searchParams.append("response_type", "code");
    auth_url.searchParams.append("redirect_uri", `${origin}/auth`);
    auth_url.searchParams.append("scope", "playlist-read-private");
    auth_url.searchParams.append("show_dialog", "false");
    return auth_url;
}

async function authenticate_spotify(
    env: Environment,
    origin: string
): Promise<Response> {
    return Response.redirect(origin, 307);
}

export default {
    async fetch(request: Request, env: Environment): Promise<Response> {
        let spotify_token = await env.SPOTIFY_BACKUP_KV.get<SpotifyKV>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        let url = new URL(request.url);
        let origin = url.origin;
        let auth_url = create_authentication_url(env, origin);

        if (request.method === "GET") {
            switch (url.pathname) {
                case "/":
                    return await fetch_home(spotify_token, auth_url);
                case "/auth":
                    return await authenticate_spotify(env, origin);
                default:
                    return new Response(`route not found`, { status: 404 });
            }
        } else {
            return new Response(`unexpected request method`, { status: 400 });
        }
    },
    async scheduled(event: ScheduledEvent, env: Environment) {
        let spotify_token = await env.SPOTIFY_BACKUP_KV.get<SpotifyKV>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        if (spotify_token === null) {
            console.warn("No spotify token :(");
            return;
        }

        const client = new spotify.Client({
            token: {
                clientID: env.SPOTIFY_CLIENT_ID,
                clientSecret: env.SPOTIFY_CLIENT_SECRET,
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
