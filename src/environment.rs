use std::{env, net::SocketAddr, path::PathBuf};

use axum::http::{uri::Authority, Uri};
use octocrab::{models::AppId, Octocrab};
use once_cell::sync::Lazy;

pub struct HttpEnvironment {
    pub bind: SocketAddr,
    pub static_dir: PathBuf,
    pub domain: Authority,
}

pub static HTTP_ENVIRONMENT: Lazy<HttpEnvironment> = Lazy::new(|| HttpEnvironment {
    bind: env::var("BIND")
        .expect("$BIND should be set")
        .parse()
        .expect("$BIND should be a valid SocketAddr"),
    static_dir: env::var_os("STATIC_DIR")
        .expect("$STATIC_DIR should be set")
        .into(),
    domain: env::var("DOMAIN")
        .expect("$DOMAIN should be set")
        .parse::<Authority>()
        .expect("$DOMAIN should be a valid URI authority"),
});

#[derive(Debug, Clone)]
pub struct SpotifyEnvironment {
    pub credentials: rspotify::Credentials,
    pub redirect_uri: Uri,
}

pub static SPOTIFY_ENVIRONMENT: Lazy<SpotifyEnvironment> = Lazy::new(|| SpotifyEnvironment {
    credentials: rspotify::Credentials {
        id: env::var("SPOTIFY_CLIENT_ID").expect("$SPOTIFY_CLIENT_ID should be set"),
        secret: Some(
            env::var("SPOTIFY_CLIENT_SECRET").expect("$SPOTIFY_CLIENT_SECRET should be set"),
        ),
    },
    redirect_uri: env::var("SPOTIFY_REDIRECT_URI")
        .expect("$SPOTIFY_REDIRECT_URI should be set")
        .parse()
        .expect("$SPOTIFY_REDIRECT_URI should be a valid URI"),
});

#[derive(Debug, Clone)]
// TODO: secrecy crate
pub struct GithubEnvironment {
    // Oauth
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: Uri,

    pub app_auth: octocrab::auth::AppAuth,
    pub client: Octocrab,
}

pub static GITHUB_ENVIRONMENT: Lazy<GithubEnvironment> = Lazy::new(|| {
    let app_id: AppId = env::var("GITHUB_APP_ID")
        .expect("$GITHUB_APP_ID should be set")
        .parse::<u64>()
        .expect("$GITHUB_APP_ID should be a valid app id")
        .into();

    let key_path: PathBuf = env::var_os("GITHUB_PRIVATE_KEY")
        .expect("$GITHUB_PRIVATE_KEY should be set")
        .into();

    let key_contents = std::fs::read(key_path).expect("$GITHUB_PRIVATE_KEY should be read in");

    let encoding_key = jsonwebtoken::EncodingKey::from_rsa_pem(&key_contents)
        .expect("encoding key should be valid RSA PEM");

    let client = Octocrab::builder()
        .app(app_id, encoding_key.clone())
        // .retry_predicate(|_| true) // TODO: this could be cool
        .build()
        .expect("github environment should be valid");

    GithubEnvironment {
        client,
        app_auth: octocrab::auth::AppAuth {
            app_id,
            key: encoding_key,
        },
        // TODO: manual oauth :(
        client_id: env::var("GITHUB_CLIENT_ID").expect("$GITHUB_CLIENT_ID should be set"),
        client_secret: env::var("GITHUB_CLIENT_SECRET")
            .expect("$GITHUB_CLIENT_SECRET should be set"),
        redirect_uri: env::var("GITHUB_REDIRECT_URI")
            .expect("$GITHUB_REDIRECT_URI should be set")
            .parse()
            .expect("$GITHUB_REDIRECT_URI should be a valid URI"),
    }
});
