import { Environment, get_origin } from "./env";
import "spotify-web-api-js";
import html_response from "./render/response";
import server_error from "./pages/error/500";

const SPOTIFY_KV_TOKEN = "spotify-token";
const SPOTIFY_ACCOUNTS = "https://accounts.spotify.com";

const SPOTIFY_TOKEN_URL = `${SPOTIFY_ACCOUNTS}/api/token`;
const SPOTIFY_AUTH_URL = `${SPOTIFY_ACCOUNTS}/authorize`;

interface OAuthResponse {
    readonly access_token: string;
    readonly token_type: "Bearer";
    readonly refresh_token: string;
    readonly scope: string;
    readonly expires_in: number;
}

interface RefreshOAuth extends Omit<OAuthResponse, "refresh_token"> {}

interface StorageOAuth extends Omit<OAuthResponse, "expires_in"> {
    readonly expires_at: number;
}

class OAuth {
    readonly storage!: StorageOAuth;

    private constructor(oauth: StorageOAuth) {
        this.storage = oauth;
    }

    public static from_response(response: OAuthResponse): OAuth {
        return new OAuth({
            ...response,
            expires_at: Date.now() + response.expires_in * 1000,
        });
    }

    public static async from_persistance(
        env: Environment
    ): Promise<OAuth | null> {
        const persistance = await env.SPOTIFY_BACKUP_KV.get<StorageOAuth>(
            SPOTIFY_KV_TOKEN,
            "json"
        );

        if (persistance === null) {
            return null;
        }

        return new OAuth(persistance);
    }

    public async persist(env: Environment) {
        await env.SPOTIFY_BACKUP_KV.put(
            SPOTIFY_KV_TOKEN,
            JSON.stringify(this.storage)
        );
    }

    public static async remove(env: Environment) {
        await env.SPOTIFY_BACKUP_KV.delete(SPOTIFY_KV_TOKEN);
    }

    public expired() {
        return Date.now() > this.storage.expires_at;
    }

    public async refresh(env: Environment): Promise<OAuth | null> {
        const response = await fetch(SPOTIFY_TOKEN_URL, {
            method: "POST",
            body: new URLSearchParams({
                grant_type: "refresh_token",
                refresh_token: this.storage.refresh_token,
            }),
            headers: {
                Authorization: `Basic ${btoa(
                    `${env.SPOTIFY_CLIENT_ID}:${env.SPOTIFY_CLIENT_SECRET}`
                )}`,
                "Content-Type": "application/x-www-form-urlencoded",
            },
        });

        if (!response.ok) {
            console.log(
                "failed to refresh access token",
                response.status,
                response.statusText
            );

            return null;
        }

        const json: RefreshOAuth = await response.json();

        return OAuth.from_response({
            refresh_token: this.storage.refresh_token,
            ...json,
        });
    }
}


function get_redirect(env: Environment) {
    return `${get_origin(env)}/auth`;
}

export function create_authentication_url(env: Environment): URL {
    let auth_url = new URL(SPOTIFY_AUTH_URL);
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
    ctx: ExecutionContext,
    searchParams: URLSearchParams
): Promise<Response> {
    // let auth_state = searchParams.get("state"); // TODO:

    const auth_error = searchParams.get("error");

    if (auth_error !== null) {
        return server_error("failed to authenticate with spotify", auth_error);
    }

    const auth_code = searchParams.get("code");

    if (auth_code === null) {
        return Response.redirect(create_authentication_url(env).toString());
    }

    const response = await fetch(SPOTIFY_TOKEN_URL, {
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
        console.log(
            "failed to request access token",
            response.status,
            response.statusText
        );

        return Response.redirect(`${get_origin(env)}/#access_token_failure`);
    }

    const oauth = OAuth.from_response(await response.json<OAuthResponse>());
    ctx.waitUntil(oauth.persist(env));

    return Response.redirect(get_origin(env));
}

export async function de_authenticate_spotify(
    env: Environment,
    ctx: ExecutionContext
) {
    ctx.waitUntil(OAuth.remove(env));

    return Response.redirect(get_origin(env));
}

export default class SpotifyClient {
    private _oauth!: OAuth;
    private env: Environment;

    private async set_oauth(oauth: OAuth) {
        this._oauth = oauth;
        this._oauth.persist(this.env);
    }

    public get oauth(): OAuth {
        return this._oauth;
    }

    private constructor(oauth: OAuth, env: Environment) {
        this._oauth = oauth;
        this.env = env;
    }

    public static async from_env(
        env: Environment
    ): Promise<SpotifyClient | null> {
        const spotify_oauth = await OAuth.from_persistance(env);

        if (spotify_oauth === null) {
            return null;
        }

        const client = new SpotifyClient(spotify_oauth, env);

        await client.check_oauth();

        return client;
    }

    private async check_oauth() {
        if (this.oauth.expired()) {
            console.log("oauth token expired");

            const refreshed_oauth = await this.oauth.refresh(this.env);

            // TODO: distinguish bad refresh vs good refresh
            if (refreshed_oauth === null) {
                console.log("failed to refresh oauth token");
                OAuth.remove(this.env);
            } else {
                this.set_oauth(refreshed_oauth);
                refreshed_oauth.persist(this.env);
            }
        }
    }

    private async fetch<T>(path: string): Promise<FetchResult<T>> {
        await this.check_oauth();

        const url = path.startsWith("http")
            ? path
            : `https://api.spotify.com/v1${path}`;

        const response = await fetch(url, {
            headers: {
                Authorization: `${this.oauth.storage.token_type} ${this.oauth.storage.access_token}`,
                Accept: "application/json",
            },
        });

        const body = await response.json<T>();

        if (!response.ok) {
            console.log(`failed to fetch "${url}"`, body);

            return {
                success: false,
                error: {
                    status: response.status,
                    statusText: response.statusText,
                    response: await response.text(),
                },
            };
        }

        return {
            success: true,
            data: body,
        };
    }

    public async me() {
        return this.fetch<SpotifyApi.CurrentUsersProfileResponse>("/me");
    }

    public async my_saved_tracks(): Promise<
        FetchResult<SpotifyApi.SavedTrackObject[]>
    > {
        const MAX_LIMIT = 50;

        const first_response =
            await this.fetch<SpotifyApi.UsersSavedTracksResponse>(
                `/me/tracks?limit=${MAX_LIMIT}`
            );

        if (!first_response.success) {
            return first_response;
        }

        // Calculate the amount of subsequent requests
        const total_tracks = first_response.data.total;
        let request_promises: Promise<
            FetchResult<SpotifyApi.UsersSavedTracksResponse>
        >[] = [];

        for (let start = MAX_LIMIT; start < total_tracks; start += MAX_LIMIT) {
            request_promises.push(
                this.fetch<SpotifyApi.UsersSavedTracksResponse>(
                    `/me/tracks?limit=${MAX_LIMIT}&offset=${start}`
                )
            );
        }

        const request_responses = await Promise.all(request_promises);

        function all_success<T>(
            arr: FetchResult<T>[]
        ): arr is FetchSuccess<T>[] {
            return arr.every((response) => response.success);
        }

        if (!all_success(request_responses)) {
            return request_responses.find(
                (response) => !response.success
            )! as FetchFailure;
        }

        // Join the first response and the subsequent responses into an array
        const saved_tracks = first_response.data.items.concat(
            request_responses.flatMap((response) => response.data.items)
        );

        // Make absolutely sure that the items are sorted in a deterministic manner
        return {
            success: true,
            data: saved_tracks.sort((a, b) => {
                // Sort by addition date
                const added_at_cmp =
                    new Date(b.added_at).getTime() -
                    new Date(a.added_at).getTime();

                if (added_at_cmp != 0) {
                    return added_at_cmp;
                }

                // Fall back if added at same time
                const track_name_cmp = a.track.name.localeCompare(b.track.name);

                if (track_name_cmp != 0) {
                    return track_name_cmp;
                }

                // Fall back again if added at same time and same name
                return a.track.album.name.localeCompare(b.track.album.name);
            }),
        };
    }
}

export type FetchFailure = {
    success: false;
    error: {
        status: number;
        statusText: string;
        response: string;
    };
};
export type FetchSuccess<T> = {
    success: true;
    data: T;
};
export type FetchResult<T> = FetchSuccess<T> | FetchFailure;
