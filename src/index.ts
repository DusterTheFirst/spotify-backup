import octokit from "octokit";
import { Environment, is_environment } from "./env";
import { fetch_home } from "./home";
import {
    authenticate_spotify,
    de_authenticate_spotify,
    spotify_client,
} from "./spotify";
import manifest from "./manifest.json";

export default {
    async fetch(
        request: Request,
        env: Partial<Environment>,
        ctx: ExecutionContext
    ): Promise<Response> {
        if (!is_environment(env)) {
            console.error("missing expected environment variables", env);
            return new Response("misconfigured worker", { status: 500 });
        }

        let url = new URL(request.url);

        let spotify = await spotify_client(env);

        if (request.method === "GET") {
            if (url.pathname.startsWith("/assets/")) {
                let response = await fetch(
                    `https://dusterthefirst.github.io/spotify-backup/${url.pathname.replace(
                        "/assets/",
                        ""
                    )}`
                );

                if (response.status === 404) {
                    return new Response("asset not found", { status: 404 });
                } else {
                    return response;
                }
            }

            switch (url.pathname) {
                case "/":
                    return fetch_home(spotify);
                case "/auth":
                    return authenticate_spotify(env, url.searchParams);
                case "/de-auth":
                    return de_authenticate_spotify(env);
                case "/manifest.json":
                    return new Response(JSON.stringify(manifest));
                default:
                    return new Response("route not found", { status: 404 });
            }
        } else {
            return new Response("unexpected request method", { status: 400 });
        }
    },
    async scheduled(
        event: ScheduledEvent,
        env: Partial<Environment>,
        ctx: ExecutionContext
    ): Promise<void> {
        if (!is_environment(env)) {
            console.error("missing expected environment variables", env);
            return;
        }

        let spotify = await spotify_client(env);

        if (spotify === null) {
            console.warn("spotify not authenticated :(");
            return;
        }

        console.log(event);
    },
};
