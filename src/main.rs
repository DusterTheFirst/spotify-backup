#![forbid(unsafe_code)]
#![deny(elided_lifetimes_in_paths)]

use std::{io, net::SocketAddr, path::PathBuf};

use axum::{
    body::Body,
    extract::Extension,
    http::{Request, StatusCode},
    response::IntoResponse,
    routing::{any_service, get, get_service},
    Router,
};
use color_eyre::eyre::Context;
use serde::Deserialize;
use tower::service_fn;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing::{debug, error, Level};
use tracing_subscriber::EnvFilter;

use crate::templates::not_found_service;

mod routes;
mod templates;

#[derive(Deserialize)]
struct Environment {
    bind: SocketAddr,
    static_dir: PathBuf,
    spotify_client_id: String,
    spotify_client_secret: String,
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_error| {
            EnvFilter::default()
                .add_directive(Level::INFO.into())
                .add_directive("tower_http=debug".parse().unwrap())
                .add_directive("spotify_backup=trace".parse().unwrap())
        }))
        .init();

    let env: Environment =
        envy::from_env().wrap_err("failed to load configuration from environment")?;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(env))
}

async fn async_main(env: Environment) -> color_eyre::Result<()> {
    let rspotify_credentials =
        rspotify::Credentials::new(&env.spotify_client_id, &env.spotify_client_secret);

    let app = Router::new()
        .layer(
            tower::ServiceBuilder::new()
                .layer(CorsLayer::permissive())
                .layer(Extension(rspotify_credentials)),
        )
        .route("/", get(routes::root))
        .route("/api/auth", get(routes::api::auth))
        .route("/api/auth/redirect", get(routes::api::auth_redirect))
        .route("/api/healthy", get(|| async { "OK" }))
        .nest(
            "/static",
            any_service(
                ServeDir::new(env.static_dir)
                    .fallback(get_service(service_fn(not_found_service::<io::Error>)))
                    .append_index_html_on_directories(false),
            )
            .handle_error(|err| async move {
                error!(%err, "ServeDir encountered IO error");

                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }),
        )
        .fallback(get_service(service_fn(not_found_service)));

    debug!("listening on http://{}", env.bind);
    axum::Server::bind(&env.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
