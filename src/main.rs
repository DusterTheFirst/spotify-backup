#![forbid(unsafe_code)]
#![deny(elided_lifetimes_in_paths)]

use std::{net::SocketAddr, path::PathBuf};

use axum::{
    routing::{get, get_service},
    Router,
};
use color_eyre::eyre::Context;
use serde::Deserialize;
use tower::service_fn;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, services::ServeDir, trace::TraceLayer,
};
use tracing::{debug, Instrument, Level};
use tracing_subscriber::{prelude::*, EnvFilter};

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
    dotenvy::dotenv().ok();

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_error| {
            EnvFilter::default()
                .add_directive(Level::INFO.into())
                .add_directive("tower_http=trace".parse().unwrap())
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
    let _rspotify_credentials =
        rspotify::Credentials::new(&env.spotify_client_id, &env.spotify_client_secret);

    let app = Router::new()
        .route("/", get(routes::root))
        .route("/api/auth", get(routes::api::auth))
        .route("/api/auth/redirect", get(routes::api::auth_redirect))
        .route("/api/healthy", get(|| async { "OK" }))
        .route(
            "/api/panic",
            get(|| {
                async { panic!("you told me to do this") }.instrument(tracing::info_span!("piss"))
            }),
        )
        .nest_service(
            "/static",
            get_service(
                ServeDir::new(env.static_dir)
                    .append_index_html_on_directories(false)
                    .fallback(service_fn(
                        routes::error::not_found_service::<std::io::Error>,
                    )),
            )
            .handle_error(routes::error::internal_server_error),
        )
        .fallback(routes::error::not_found)
        .layer(
            tower::ServiceBuilder::new()
                .layer(CatchPanicLayer::custom(
                    routes::error::internal_server_error_panic,
                ))
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive()),
        );

    debug!(?env.bind, "started http server");
    axum::Server::bind(&env.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
