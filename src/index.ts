import octokit from "octokit";
import { Environment, is_environment } from "./env";
import { fetch_home } from "./home";
import {
    authenticate_spotify,
    de_authenticate_spotify,
    spotify_client,
} from "./spotify";

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
            switch (url.pathname) {
                case "/":
                    return await fetch_home(spotify);
                case "/auth":
                    return await authenticate_spotify(env, url.searchParams);
                case "/de-auth":
                    return await de_authenticate_spotify(env);
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
