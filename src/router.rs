use std::time::Duration;

use axum::{extract::FromRef, http::header, response::Redirect, routing::get, Router};
use color_eyre::eyre::Context;
use tower_http::{
    cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir, timeout::TimeoutLayer,
    trace::TraceLayer, ServiceBuilderExt,
};
use tracing::debug;

use middleware::{catch_panic::catch_panic_layer, trace::SpanMaker};

use crate::{
    database::Database, environment::HTTP_ENVIRONMENT, pages,
    router::middleware::server_information::StaticServerInformation,
};

pub mod authentication;
pub mod error;
pub mod middleware;
pub mod session;

pub async fn favicon() -> Redirect {
    Redirect::to("/static/branding/logo@192.png")
}

#[derive(FromRef, Clone)]
pub struct AppState {
    pub database: Database,
    pub reqwest: reqwest::Client,
}

pub async fn router(database: Database) -> color_eyre::Result<()> {
    let state = AppState {
        database,
        reqwest: reqwest::Client::builder()
            .brotli(true)
            .gzip(true)
            .deflate(true)
            .https_only(true)
            .use_rustls_tls()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION"),
            ))
            .build()
            .expect("failed to create reqwest client"),
    };

    let app = Router::new()
        .route("/", get(pages::home))
        .route("/dashboard", get(pages::dashboard))
        .route("/account", get(pages::account))
        .route("/login/spotify", get(authentication::spotify::login))
        .route("/login/github", get(authentication::github::login))
        .route("/logout", get(authentication::logout))
        .route("/logout/delete", get(authentication::delete))
        .route("/logout/github", get(authentication::github::logout))
        // TODO: Image resizing/optimization
        .route("/favicon.ico", get(favicon))
        .route("/health", get(|| async { "OK" }))
        .nest_service(
            "/static",
            ServeDir::new(&HTTP_ENVIRONMENT.static_dir)
                .append_index_html_on_directories(false)
                .call_fallback_on_method_not_allowed(true)
                .fallback(get(error::not_found)),
        )
        .fallback(error::not_found)
        .layer(
            tower::ServiceBuilder::new()
                // Hide sensitive headers
                .sensitive_headers([header::AUTHORIZATION, header::COOKIE])
                // Give a unique identifier to every request
                .set_x_request_id(MakeRequestUuid) // TODO: USE
                .propagate_x_request_id()
                // Trace requests and responses
                .layer(TraceLayer::new_for_http().make_span_with(SpanMaker)) // TODO: configure
                // Send traces to sentry
                .layer(sentry::integrations::tower::NewSentryLayer::new_from_top())
                .layer(sentry::integrations::tower::SentryHttpLayer::with_transaction())
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
                        .allow_origin([HTTP_ENVIRONMENT
                            .domain
                            .as_str()
                            .parse()
                            .expect("a URI should be a valid HeaderValue")]),
                )
                // Redirect requests that are not to the configured domain
                .layer(axum::middleware::from_fn_with_state(
                    HTTP_ENVIRONMENT.domain.clone(),
                    middleware::redirect::redirect_to_domain,
                ))
                // Set server information response headers
                .layer(axum::middleware::from_fn(
                    StaticServerInformation::middleware,
                ))
                // Catch Panics in handlers
                .layer(catch_panic_layer(error::internal_server_error_panic)),
        )
        .with_state(state);

    debug!(bind = ?HTTP_ENVIRONMENT.bind, "started http server");
    axum::Server::bind(&HTTP_ENVIRONMENT.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
