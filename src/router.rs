use std::time::Duration;

use axum::{http::header, response::Redirect, routing::get, Router};
use color_eyre::eyre::Context;
use rspotify::scopes;
use tower_http::{
    cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir, timeout::TimeoutLayer,
    trace::TraceLayer, ServiceBuilderExt,
};
use tracing::debug;

use middleware::{catch_panic::catch_panic_layer, trace::SpanMaker};

use crate::pages;

use super::{GithubEnvironment, HttpEnvironment, SpotifyEnvironment};

pub mod authentication;
pub mod error;
pub mod middleware;

pub async fn favicon() -> Redirect {
    Redirect::to("/static/branding/logo@192.png")
}

pub async fn router(
    http: HttpEnvironment,
    spotify: SpotifyEnvironment,
    github: GithubEnvironment,
) -> color_eyre::Result<()> {
    let rspotify_oauth = rspotify::OAuth {
        redirect_uri: spotify.redirect_uri.to_string(),
        scopes: scopes!("playlist-read-private", "user-library-read"),
        ..Default::default()
    };

    let app = Router::new()
        .route("/", get(pages::dashboard))
        .route("/login", get(pages::login))
        .route(
            "/login/spotify",
            get(authentication::login_spotify).with_state(spotify.credentials),
        )
        .route("/login/github", get(authentication::login_github))
        // TODO: Image resizing/optimization
        .route("/favicon.ico", get(favicon))
        .route("/health", get(|| async { "OK" }))
        .nest_service(
            "/static",
            ServeDir::new(http.static_dir)
                .append_index_html_on_directories(false)
                .call_fallback_on_method_not_allowed(true),
        )
        .fallback(error::not_found)
        .layer(
            tower::ServiceBuilder::new()
                // Hide sensitive headers
                .sensitive_headers([header::AUTHORIZATION, header::COOKIE])
                // Give a unique identifier to every request
                .propagate_x_request_id()
                .set_x_request_id(MakeRequestUuid) // TODO: USE
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
                        .allow_origin([http
                            .domain
                            .as_str()
                            .parse()
                            .expect("a URI should be a valid HeaderValue")]),
                )
                // Redirect requests that are not to the configured domain
                .layer(axum::middleware::from_fn_with_state(
                    http.domain,
                    middleware::redirect_to_domain,
                ))
                // Catch Panics in handlers
                .layer(catch_panic_layer(error::internal_server_error_panic)),
        );

    debug!(?http.bind, "started http server");
    axum::Server::bind(&http.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
