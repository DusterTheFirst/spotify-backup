use std::{convert::Infallible, env, net::SocketAddr};

use anyhow::Context;
use async_std::{
    stream::StreamExt,
    task::{self},
};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Response, Server, StatusCode,
};
use indoc::indoc;
use rspotify::{
    clients::OAuthClient, model::SavedTrack, scopes, AuthCodeSpotify, Credentials, OAuth,
};
use tracing::info;

const REDIRECT_URL: &str = "http://localhost:8080";

fn main() -> anyhow::Result<()> {
    task::block_on(start())
}

async fn start() -> anyhow::Result<()> {
    // TODO: CACHE + SAVE LOGIN
    // TODO: MAKE BETTER
    let ci = env::var("CI").is_ok();

    tracing_subscriber::fmt().init();

    let creds = Credentials::from_env().context("no rspotify credentials")?;
    let oauth = OAuth {
        scopes: scopes!("user-library-read"),
        redirect_uri: REDIRECT_URL.into(),
        ..Default::default()
    };

    let mut spotify = AuthCodeSpotify::new(creds, oauth);

    let (auth_code_tx, auth_code_rx) = flume::bounded::<String>(1);

    let (web_server_shutdown_tx, web_server_shutdown_rx) = flume::bounded::<()>(0);

    let server = Server::bind(&SocketAddr::from(([127, 0, 0, 1], 8080))).serve(make_service_fn(move |target| {
            let web_server_shutdown_tx = web_server_shutdown_tx.clone();
            let auth_code_tx = auth_code_tx.clone();

            async move {
                Ok::<_, Infallible>(service_fn(move |request| {
                    let web_server_shutdown_tx = web_server_shutdown_tx.clone();
                    let auth_code_tx = auth_code_tx.clone();

                    async move {
                        auth_code_tx.send_async(format!("{}{}", REDIRECT_URL, request.uri())).await.unwrap();

                        web_server_shutdown_tx.send_async(()).await.unwrap();

                        Response::builder()
                            .status(StatusCode::OK)
                            .header("Content-Type", "text/html")
                            .body(Body::from(indoc!{"
                                <html>
                                    <head>
                                        <script>
                                            setTimeout(() => window.close(), 1000);
                                        </script>
                                    </head>
                                    <body>
                                        <center>This page will automatically close in 1 second</center>
                                    </body>
                                </html>
                            "}))  
                    }
                }))
            }
        }));

    webbrowser::open(&spotify.get_authorize_url(false)?)?;

    server
        .with_graceful_shutdown(async move {
            web_server_shutdown_rx.recv_async().await.unwrap();
        })
        .await
        .unwrap();

    let auth_code = auth_code_rx.recv()?;

    let auth_code = spotify
        .parse_response_code(&auth_code)
        .context("failed to parse auth code")?;

    spotify.request_token(&auth_code).await?;

    let mut liked_songs = spotify.current_user_saved_tracks(None);

    let mut csv = csv::Writer::from_path("./liked_songs.csv")?;

    csv.write_record([
        "added at",
        "release date",
        "name",
        "album",
        "artist(s)",
        "popularity",
        "id",
    ])?;

    while let Some(song) = liked_songs.next().await {
        let SavedTrack { added_at, track } = song?;

        csv.write_record([
            added_at.to_rfc3339(),
            track.album.release_date.unwrap_or_default(),
            track.name,
            track.album.name,
            track
                .artists
                .iter()
                .map(|artist| artist.name.as_str())
                .collect::<Vec<&str>>()
                .join("+"),
            track.popularity.to_string(),
            track.id.to_string(),
        ])?;
    }

    Ok(())
}
