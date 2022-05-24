import { Environment } from "./env";
import "spotify-web-api-js";

const SPOTIFY_KV_TOKEN = "spotify-token";
const SPOTIFY_ACCOUNTS = "https://accounts.spotify.com/";

export interface OAuthBase {
    readonly access_token: string;
    readonly token_type: "Bearer";
    readonly refresh_token: string;
    readonly scope: string;
}

export interface OAuthResponse extends OAuthBase {
    readonly expires_in: number;
}

export interface OAuth extends OAuthBase {
    readonly expires_at: number;
}

function get_origin(env: Environment) {
    if (env.ENVIRONMENT === "dev") {
        return "http://localhost:8787";
    } else {
        return "https://spotify-backup.dusterthefirst.com";
    }
}

function get_redirect(env: Environment) {
    return `${get_origin(env)}/auth`;
}

export function create_authentication_url(env: Environment): URL {
    let auth_url = new URL(SPOTIFY_ACCOUNTS);
    auth_url.pathname = "/authorize";
    auth_url.searchParams.append("client_id", env.SPOTIFY_CLIENT_ID);
    auth_url.searchParams.append("response_type", "code");
    auth_url.searchParams.append("redirect_uri", get_redirect(env));
    auth_url.searchParams.append(
        "scope",
        ["playlist-read-private", "user-library-read"].join(" ")
    );
    auth_url.searchParams.append("show_dialog", "false");
    return auth_url;
}

export async function authenticate_spotify(
    env: Environment,
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
            create_authentication_url(env).toString(),
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
            redirect_uri: get_redirect(env),
        }),
        headers: {
            Authorization: `Basic ${btoa(
                `${env.SPOTIFY_CLIENT_ID}:${env.SPOTIFY_CLIENT_SECRET}`
            )}`,
            "Content-Type": "application/x-www-form-urlencoded",
        },
    });

    if (!response.ok) {
        console.error(
            "failed to request access token",
            response.status,
            response.statusText
        );

        return Response.redirect(
            `${get_origin(env)}/#access_token_failure`,
            307
        );
    }

    let json: OAuthResponse = await response.json();

    let oauth: OAuth = {
        expires_at: Date.now() + json.expires_in * 1000,
        access_token: json.access_token,
        refresh_token: json.refresh_token,
        scope: json.scope,
        token_type: json.token_type,
    };

    await env.SPOTIFY_BACKUP_KV.put(SPOTIFY_KV_TOKEN, JSON.stringify(oauth));

    return Response.redirect(get_origin(env), 307);
}

export async function de_authenticate_spotify(env: Environment) {
    await env.SPOTIFY_BACKUP_KV.delete(SPOTIFY_KV_TOKEN);

    return Response.redirect(get_origin(env), 307);
}

export async function spotify_client(
    env: Environment
): Promise<SpotifyClient | null> {
    let spotify_oauth = await env.SPOTIFY_BACKUP_KV.get<OAuth>(
        SPOTIFY_KV_TOKEN,
        "json"
    );

    if (spotify_oauth === null) {
        return null;
    }

    // TODO: refresh token

    return new SpotifyClient(spotify_oauth);
}

export class SpotifyClient {
    public readonly oauth!: OAuth;

    constructor(oauth: OAuth) {
        this.oauth = oauth;
    }

    // TODO: error handle
    private async fetch<T>(path: string): Promise<T> {
        return await (
            await fetch(`https://api.spotify.com/v1${path}`, {
                headers: {
                    Authorization: `${this.oauth.token_type} ${this.oauth.access_token}`,
                    Accept: "application/json",
                },
            })
        ).json();
    }

    public async me(): Promise<SpotifyApi.CurrentUsersProfileResponse> {
        return this.fetch("/me");
    }
}
