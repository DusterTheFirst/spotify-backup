import { Environment, is_environment } from "./env";
import SpotifyClient, {
    authenticate_spotify,
    de_authenticate_spotify,
} from "./spotify";
import manifest from "./manifest.json";
import { dry_run, wet_run as real_wet_run } from "./backup";
import home from "./pages/home";
import wet_run from "./pages/wet-run";
import method_not_allowed from "./pages/error/405";
import not_found from "./pages/error/404";
import push_uptime from "./uptime-kuma";

// IMPORTANT TODO: metrics and a way to tell when this starts to fail

export default {
    async fetch(
        request: Request,
        env: Partial<Environment>,
        ctx: ExecutionContext
    ): Promise<Response> {
        if (!is_environment(env)) {
            console.log("missing expected environment variables", env);
            return new Response("misconfigured worker", { status: 500 });
        }

        const url = new URL(request.url);

        const spotify = await SpotifyClient.from_env(env);

        if (request.method === "GET") {
            // Forward assets requests to github pages for static assets
            if (url.pathname.startsWith("/assets/")) {
                let response = await fetch(
                    `https://dusterthefirst.github.io/spotify-backup/${url.pathname.replace(
                        "/assets/",
                        ""
                    )}`
                );

                // If 404, pass on to other handlers
                if (response.status !== 404) {
                    return response;
                }
            }

            switch (url.pathname) {
                case "/":
                    return await home(spotify);
                case "/dry-run":
                    return dry_run(spotify, env);
                case "/wet-run":
                    return wet_run();
                case "/wet-run/no-really":
                    return real_wet_run(spotify, env);
                case "/auth":
                    return authenticate_spotify(env, ctx, url.searchParams);
                case "/de-auth":
                    return de_authenticate_spotify(env, ctx);
                case "/manifest.json":
                    return new Response(JSON.stringify(manifest), {
                        headers: { "Content-Type": "application/json" },
                    });
                default:
                    return not_found();
            }
        } else {
            return method_not_allowed(["GET"]);
        }
    },
    async scheduled(
        event: ScheduledEvent,
        env: Partial<Environment>,
        ctx: ExecutionContext
    ): Promise<void> {
        if (!is_environment(env)) {
            console.log("missing expected environment variables", env);
            return;
        }

        // Report worker as up to uptime kuma
        push_uptime(ctx);

        const spotify = await SpotifyClient.from_env(env);

        if (spotify === null) {
            console.log("spotify not authenticated :(");
            return;
        }

        await real_wet_run(spotify, env);

        console.log(event);
    },
};
