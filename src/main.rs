#![forbid(unsafe_code)]
#![deny(
    elided_lifetimes_in_paths,
    clippy::unwrap_used,
    clippy::unwrap_in_result
)]

use std::{borrow::Cow, env, net::SocketAddr, path::PathBuf, time::Duration};

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
use tracing::{debug, warn, Instrument};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::middleware::{catch_panic::catch_panic_layer, trace::SpanMaker};

mod middleware;
mod routes;
mod templates;

#[derive(Deserialize)]
struct Environment {
    #[serde(flatten)]
    http: HttpEnvironment,
    #[serde(flatten)]
    spotify: SpotifyEnvironment,
    #[serde(flatten)]
    github: GithubEnvironment,
    sentry_dsn: Option<String>,
}

#[derive(Deserialize)]
struct HttpEnvironment {
    bind: SocketAddr,
    host: String,
    static_dir: PathBuf,
}

#[derive(Deserialize)]
struct SpotifyEnvironment {
    spotify_client_id: String,
    spotify_client_secret: String,
}

#[derive(Deserialize)]
struct GithubEnvironment {}

fn main() -> Result<(), color_eyre::Report> {
    #[cfg(debug_assertions)]
    dotenvy::dotenv().ok();

    color_eyre::install()?;

    let environment: Environment =
        envy::from_env().wrap_err("failed to load configuration from environment")?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .with(sentry::integrations::tracing::layer())
        .init();

    let sentry_dsn = environment
        .sentry_dsn
        .map(|dsn| dsn.parse())
        .transpose()
        .wrap_err("SENTRY_DSN should be a valid DSN")?;

    if sentry_dsn.is_none() {
        warn!("No SENTRY_DSN provided, not reporting errors to sentry");
    }

    let _guard = sentry::init(sentry::ClientOptions {
        dsn: sentry_dsn,
        release: sentry::release_name!(), // TODO: use git hash?
        sample_rate: 1.0,
        traces_sample_rate: 1.0,
        // traces_sampler: todo!(), TODO: Do not send too many traces
        enable_profiling: true,
        profiles_sample_rate: 1.0,
        attach_stacktrace: true,
        send_default_pii: true,
        server_name: env::var("FLY_REGION").map(Cow::from).ok(),
        in_app_include: vec!["spotify_backup"],
        // in_app_exclude: todo!(),
        // auto_session_tracking: true,
        session_mode: sentry::SessionMode::Request,
        trim_backtraces: true,
        ..Default::default()
    });

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builder should succeed")
        .block_on(async_main(
            environment.http,
            environment.spotify,
            environment.github,
        ))
}

async fn async_main(
    http: HttpEnvironment,
    spotify: SpotifyEnvironment,
    github: GithubEnvironment,
) -> color_eyre::Result<()> {
    let rspotify_credentials =
        rspotify::Credentials::new(&spotify.spotify_client_id, &spotify.spotify_client_secret);

    let host =
        Authority::from_maybe_shared(http.host).expect("DOMAIN should be a valid URI authority");

    let app = Router::new()
        .route("/", get(routes::root))
        .route("/api/auth", get(routes::api::auth))
        .route("/api/auth/redirect", get(routes::api::auth_redirect))
        .route("/api/healthy", get(routes::api::healthy))
        .route("/api/panic", get(routes::api::panic))
        .nest_service(
            "/static",
            get_service(
                ServeDir::new(http.static_dir)
                    .append_index_html_on_directories(false)
                    .fallback(service_fn(
                        routes::error::static_not_found::<std::io::Error>,
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
                        .allow_origin([host
                            .as_str()
                            .parse()
                            .expect("HOST should be a valid HeaderValue")]),
                )
                // Redirect requests that are not to the configured domain
                .layer(axum::middleware::from_fn_with_state(
                    host,
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
