#![forbid(unsafe_code)]
#![deny(
    elided_lifetimes_in_paths,
    clippy::unwrap_used,
    clippy::unwrap_in_result
)]

use std::{borrow::Cow, env, net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    http::{header, uri::Authority},
    routing::get,
    Router,
};
use color_eyre::eyre::Context;
use serde::Deserialize;
use tower_http::{
    cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir, timeout::TimeoutLayer,
    trace::TraceLayer, ServiceBuilderExt,
};
use tracing::{debug, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::middleware::{catch_panic::catch_panic_layer, trace::SpanMaker};

mod middleware;
mod pages;
mod routes;

struct HttpEnvironment {
    bind: SocketAddr,
    host: Authority,
    static_dir: PathBuf,
}

struct SpotifyEnvironment {
    spotify_client_id: String,
    spotify_client_secret: String,
}

#[derive(Deserialize)]
struct GithubEnvironment {}

fn main() -> Result<(), color_eyre::Report> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .with(sentry::integrations::tracing::layer())
        .init();

    let sentry_dsn = env::var("SENTRY_DSN")
        .expect("$SENTRY_DSN must be set")
        .parse()
        .wrap_err("SENTRY_DSN should be a valid DSN")?;

    let _guard = sentry::init(sentry::ClientOptions {
        dsn: Some(sentry_dsn),
        // TODO: setup release tracking
        release: Some(git_version::git_version!(args = ["--always", "--abbrev=40"]).into()), // sentry::release_name!(), // TODO: use git hash?
        sample_rate: 1.0,
        traces_sample_rate: 0.0, // TODO: make not 0, but also not spammy
        enable_profiling: true,
        profiles_sample_rate: 1.0,
        attach_stacktrace: true,
        send_default_pii: true,
        server_name: env::var("FLY_REGION").map(Cow::from).ok(),
        in_app_include: vec!["spotify_backup"],
        // in_app_exclude: todo!(),
        auto_session_tracking: true,
        session_mode: sentry::SessionMode::Request,
        trim_backtraces: true,
        ..Default::default()
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builder should succeed")
        .block_on(async_main(
            HttpEnvironment {
                bind: env::var("BIND")
                    .expect("$BIND should be set")
                    .parse()
                    .expect("$BIND should be a valid SocketAddr"),
                host: env::var("HOST")
                    .expect("$HOST should be set")
                    .parse()
                    .expect("$HOST should be a valid URI authority"),
                static_dir: env::var_os("STATIC_DIR")
                    .expect("$STATIC_DIR should be set")
                    .into(),
            },
            SpotifyEnvironment {
                spotify_client_id: env::var("SPOTIFY_CLIENT_ID")
                    .expect("$SPOTIFY_CLIENT_ID should be set"),
                spotify_client_secret: env::var("SPOTIFY_CLIENT_SECRET")
                    .expect("$SPOTIFY_CLIENT_SECRET should be set"),
            },
            GithubEnvironment {},
        ))
}

async fn async_main(
    http: HttpEnvironment,
    spotify: SpotifyEnvironment,
    github: GithubEnvironment,
) -> color_eyre::Result<()> {
    let rspotify_credentials =
        rspotify::Credentials::new(&spotify.spotify_client_id, &spotify.spotify_client_secret);

    let api_router = Router::new()
        .route("/auth", get(routes::api::auth))
        .route("/auth/redirect", get(routes::api::auth_redirect))
        .route("/healthy", get(routes::api::healthy))
        .route("/panic", {
            if cfg!(debug_assertions) {
                get(routes::api::panic)
            } else {
                get(routes::error::not_found)
            }
        })
        .fallback(routes::api::not_found);

    let app = Router::new()
        .route("/", get(routes::index::index))
        .nest("/api/", api_router)
        // TODO: Image resizing/optimization
        .route("/favicon.ico", get(routes::favicon))
        .nest_service(
            "/static",
            ServeDir::new(http.static_dir)
                .append_index_html_on_directories(false)
                .call_fallback_on_method_not_allowed(true),
        )
        .fallback(routes::error::not_found)
        .layer(
            tower::ServiceBuilder::new()
                // Hide sensitive headers
                .sensitive_headers([header::AUTHORIZATION, header::COOKIE])
                // Give a unique identifier to every request
                .propagate_x_request_id()
                .set_x_request_id(MakeRequestUuid) // TODO: USE
                // Send traces to sentry
                .layer(sentry::integrations::tower::NewSentryLayer::new_from_top())
                .layer(sentry::integrations::tower::SentryHttpLayer::with_transaction())
                // Trace requests and responses
                .layer(TraceLayer::new_for_http().make_span_with(SpanMaker)) // TODO: configure
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
                        .allow_origin([http
                            .host
                            .as_str()
                            .parse()
                            .expect("HOST should be a valid HeaderValue")]),
                )
                // Redirect requests that are not to the configured domain
                .layer(axum::middleware::from_fn_with_state(
                    http.host,
                    middleware::redirect_to_domain,
                ))
                // Catch Panics in handlers
                .layer(catch_panic_layer(
                    routes::error::internal_server_error_panic,
                )),
        );

    debug!(?http.bind, "started http server");
    axum::Server::bind(&http.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
