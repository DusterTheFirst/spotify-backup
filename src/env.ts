export interface Environment {
    readonly SPOTIFY_BACKUP_KV: KVNamespace;
    readonly SPOTIFY_CLIENT_ID: string;
    readonly SPOTIFY_CLIENT_SECRET: string;
    readonly GITHUB_ACCESS_TOKEN: string;
    readonly ENVIRONMENT: "dev" | undefined;
}

export function is_environment(env: Partial<Environment>): env is Environment {
    return (
        env.SPOTIFY_BACKUP_KV !== undefined &&
        env.SPOTIFY_CLIENT_ID !== undefined &&
        env.SPOTIFY_CLIENT_SECRET !== undefined &&
        env.GITHUB_ACCESS_TOKEN !== undefined
    );
}

export function get_origin(env: Environment) {
    if (env.ENVIRONMENT === "dev") {
        return "http://localhost:8787";
    } else {
        return "https://spotify-backup.dusterthefirst.com";
    }
}
