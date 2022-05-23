import octokit from "octokit";
import spotify from "spotify-api.js";

interface Environment {
    readonly SPOTIFY_BACKUP_KV: KVNamespace;
    readonly SPOTIFY_CLIENT_ID: string;
    readonly SPOTIFY_CLIENT_SECRET: string;
    readonly ENVIRONMENT: "dev" | undefined;
}
const SPOTIFY_KV_TOKEN = "spotify-token";
interface SpotifyOAuth {
    readonly access_token: string;
    readonly token_type: "Bearer";
    readonly expires_in: number;
    readonly refresh_token: string;
    readonly scope: string;
}

const SPOTIFY_API = "https://api.spotify.com/";
const SPOTIFY_ACCOUNTS = "https://accounts.spotify.com/";

function html_response(body: string, head: string = "") {
    return new Response(
        `<!DOCTYPE html><head>${head}</head><body>${body}</body>`,
        {
            headers: {
                "content-type": "text/html;charset=UTF-8",
            },
        }
    );
}

async function fetch_home(spotify_token: SpotifyOAuth | null) {
    if (spotify_token !== null) {
        return html_response(
            `<span style="color: green">user authenticated</span><br/><a href="/auth">re-authenticate</a><br/><a href="/de-auth">de-authenticate</a>`
        );
    }

    return html_response(
        `<span style="color: red">user not authenticated</span><br/><a href="/auth">authenticate</a>`
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
    origin: string,
    searchParams: URLSearchParams
): Promise<Response> {
    // let auth_state = searchParams.get("state"); // TODO:

    let auth_error = searchParams.get("error");

    if (auth_error !== null) {
        return new Response(`encountered an error: ${auth_error}`, {
            status: 400,
        });
    }

    let auth_code = searchParams.get("code");

    if (auth_code === null) {
        return Response.redirect(
            create_authentication_url(env, origin).toString(),
            307
        );
    }

    let token_url = new URL(SPOTIFY_ACCOUNTS);
    token_url.pathname = "/api/token";
    let response = await fetch(token_url.toString(), {
        method: "POST",
        body: new URLSearchParams({
            grant_type: "authorization_code",
            code: auth_code,
            redirect_uri: `${origin}/auth`,
        }),
        headers: {
            Authorization: `Basic ${btoa(
                `${env.SPOTIFY_CLIENT_ID}:${env.SPOTIFY_CLIENT_SECRET}`
            )}`,
        },
    });

    if (!response.ok) {
        console.warn(
            "failed to request access token",
            response.status,
            response.statusText
        );

        return Response.redirect(`${origin}/#access_token_failure`, 307);
    }

    let json = await response.json();

    await env.SPOTIFY_BACKUP_KV.put(SPOTIFY_KV_TOKEN, JSON.stringify(json));

    return Response.redirect(origin, 307);
}

async function de_authenticate_spotify(env: Environment, origin: string) {
    await env.SPOTIFY_BACKUP_KV.delete(SPOTIFY_KV_TOKEN);

    return Response.redirect(origin, 307);
}

export default {
    async fetch(request: Request, env: Environment): Promise<Response> {
        let spotify_token = await env.SPOTIFY_BACKUP_KV.get<SpotifyOAuth>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        console.log(env);

        let url = new URL(request.url);
        let origin;
        if (env.ENVIRONMENT === "dev") {
            origin = "http://localhost:8787";
        } else {
            origin = url.origin;
        }

        if (request.method === "GET") {
            switch (url.pathname) {
                case "/":
                    return await fetch_home(spotify_token);
                case "/auth":
                    return await authenticate_spotify(
                        env,
                        origin,
                        url.searchParams
                    );
                case "/de-auth":
                    return await de_authenticate_spotify(env, origin);
                default:
                    return new Response("route not found", { status: 404 });
            }
        } else {
            return new Response("unexpected request method", { status: 400 });
        }
    },
    async scheduled(event: ScheduledEvent, env: Environment) {
        let spotify_token = await env.SPOTIFY_BACKUP_KV.get<SpotifyOAuth>(
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
                token: spotify_token.access_token,
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
