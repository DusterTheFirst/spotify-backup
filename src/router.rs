use std::time::Duration;

use axum::{http::header, response::Redirect, routing::get, Router};
use color_eyre::eyre::Context;
use tower_http::{
    cors::CorsLayer, request_id::MakeRequestUuid, services::ServeDir, timeout::TimeoutLayer,
    trace::TraceLayer, ServiceBuilderExt,
};
use tracing::debug;

use middleware::{catch_panic::catch_panic_layer, trace::SpanMaker};

use super::{GithubEnvironment, HttpEnvironment, SpotifyEnvironment};

pub mod api;
pub mod error;
pub mod index;
pub mod login;
pub mod middleware;

pub async fn favicon() -> Redirect {
    Redirect::to("/static/branding/logo@192.png")
}

pub async fn router(
    http: HttpEnvironment,
    spotify: SpotifyEnvironment,
    github: GithubEnvironment,
) -> color_eyre::Result<()> {
    let rspotify_credentials =
        rspotify::Credentials::new(&spotify.spotify_client_id, &spotify.spotify_client_secret);

    let api_router = Router::new()
        .route("/auth", get(api::auth))
        .route("/auth/redirect", get(api::auth_redirect))
        .route("/healthy", get(api::healthy))
        .route("/panic", {
            if cfg!(debug_assertions) {
                get(api::panic)
            } else {
                get(error::not_found)
            }
        })
        .fallback(api::not_found);

    let app = Router::new()
        .route("/", get(index::index))
        .nest("/api/", api_router)
        // TODO: Image resizing/optimization
        .route("/favicon.ico", get(favicon))
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
                .layer(catch_panic_layer(error::internal_server_error_panic)),
        );

    debug!(?http.bind, "started http server");
    axum::Server::bind(&http.bind)
        .serve(app.into_make_service())
        .await
        .wrap_err("failed to bind to given address")
}
