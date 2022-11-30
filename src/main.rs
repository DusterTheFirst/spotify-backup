#![forbid(unsafe_code)]
#![deny(elided_lifetimes_in_paths)]

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    http::header::{self},
    routing::{get, get_service},
    Router,
};
use color_eyre::eyre::Context;
use serde::Deserialize;
use tower::service_fn;
use tower_http::{
    catch_panic::CatchPanicLayer, cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir,
    timeout::TimeoutLayer, trace::TraceLayer, ServiceBuilderExt,
};
use tracing::{debug, Instrument, Level};
use tracing_subscriber::{prelude::*, EnvFilter};

mod routes;
mod templates;

#[derive(Deserialize)]
struct Environment {
    bind: SocketAddr,
    domain: String,
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
                .sensitive_headers([header::AUTHORIZATION, header::COOKIE])
                .set_x_request_id(MakeRequestUuid) // TODO: USE
                .propagate_x_request_id()
                .layer(TraceLayer::new_for_http()) // TODO: configure
                .layer(TimeoutLayer::new(Duration::from_secs(10)))
                .map_response_body(axum::body::boxed)
                .compression()
                // .layer( TODO: redirect to environment.domain
                //     FilterLayer::new(|req: Request<Body>| {
                //         req.uri()
                //             .authority()
                //             .map(|auth| auth.as_str())
                //             .eq(&Some(&environment.domain))
                //             .then_some(req)
                //             .ok_or(req)
                //     })
                //     .layer(Redirect::<BoxBody>::permanent(
                //         environment
                //             .domain
                //             .parse()
                //             .expect("domain should be a valid uri"),
                //     )),
                // )
                .layer(CatchPanicLayer::custom(
                    routes::error::internal_server_error_panic,
                ))
                .layer(CorsLayer::permissive()), // TODO: less permissive
        );

    debug!(?environment.bind, "started http server");
    axum::Server::bind(&environment.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
