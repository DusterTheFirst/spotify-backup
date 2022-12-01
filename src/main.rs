#![forbid(unsafe_code)]
#![deny(elided_lifetimes_in_paths)]

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    http::{header, uri::Authority},
    routing::{get, get_service},
    Router,
};
use color_eyre::eyre::Context;
use serde::Deserialize;
use tower::service_fn;
use tower_http::{
    cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir, timeout::TimeoutLayer,
    trace::TraceLayer, ServiceBuilderExt,
};
use tracing::{debug, Instrument, Level};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::middleware::catch_panic;

mod middleware;
mod routes;
mod templates;

#[derive(Deserialize)]
struct Environment {
    bind: SocketAddr,
    host: String,
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

    let environment: Environment =
        envy::from_env().wrap_err("failed to load configuration from environment")?;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async_main(environment))
}

async fn async_main(environment: Environment) -> color_eyre::Result<()> {
    let rspotify_credentials = rspotify::Credentials::new(
        &environment.spotify_client_id,
        &environment.spotify_client_secret,
    );

    let host = Authority::from_maybe_shared(environment.host)
        .expect("DOMAIN should be a valid URI authority");

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
                ServeDir::new(environment.static_dir)
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
                // Hide sensitive headers
                .sensitive_headers([header::AUTHORIZATION, header::COOKIE])
                // Give a unique identifier to every request
                .propagate_x_request_id()
                .set_x_request_id(MakeRequestUuid) // TODO: USE
                // Catch Panics in handlers
                .layer(axum::middleware::from_fn(catch_panic))
                // Trace requests and responses
                .layer(TraceLayer::new_for_http()) // TODO: configure
                // Timeout if request or response hangs
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                // Compress responses
                .map_response_body(axum::body::boxed)
                .compression()
                // Send CORS headers
                // TODO: less restrictive
                .layer(
                    CorsLayer::new()
                        .allow_credentials(false)
                        .allow_headers([])
                        .allow_methods([])
                        .allow_origin([host
                            .as_str()
                            .parse()
                            .expect("HOST should be a valid HeaderValue")]),
                )
                // Redirect requests that are not to the configured domain
                .layer(axum::middleware::from_fn_with_state(
                    host,
                    middleware::redirect_to_domain,
                )),
        );

    debug!(?environment.bind, "started http server");
    axum::Server::bind(&environment.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
