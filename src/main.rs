use serde::{Deserialize, Serialize};
use tracing::info;
use tracing_subscriber::EnvFilter;

struct Secrets {
    github_access_token: String,
    spotify_access_token: String,
}

#[derive(Serialize, Deserialize)]
struct Backup {
    github: String,
    repository: String,
    spotify: String,
}

fn main() {
    tracing_subscriber::fmt()
        .compact()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Hello, world!");
}
