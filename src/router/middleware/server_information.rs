use axum::{
    body::Body,
    http::{header, HeaderName, HeaderValue, Request},
    middleware::Next,
    response::IntoResponse,
};
use git_version::git_version;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StaticServerInformation {
    /// Server name
    pub name: &'static str,
    /// Server SemVer version
    pub version: &'static str,
    /// Server git commit
    pub commit: &'static str,
    /// Server source code URI
    pub source: &'static str,
    /// Server environment (dev/prod)
    pub environment: &'static str,
}

impl StaticServerInformation {
    const SERVER: Self = StaticServerInformation {
        name: env!("CARGO_PKG_NAME"),
        version: env!("CARGO_PKG_VERSION"),
        commit: git_version!(),
        source: const_format::formatcp!(
            "https://github.com/dusterthefirst/spotify-backup/tree/{}",
            git_version!(args = ["--always"])
        ),
        environment: if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        },
    };

    pub async fn middleware(req: Request<Body>, next: Next<Body>) -> impl IntoResponse {
        (
            axum::response::AppendHeaders([
                (
                    HeaderName::from_static("x-origin-server"),
                    HeaderValue::from_static(const_format::formatcp!(
                        "{}@{} (commit {})",
                        StaticServerInformation::SERVER.name,
                        StaticServerInformation::SERVER.version,
                        StaticServerInformation::SERVER.commit
                    )),
                ),
                (
                    HeaderName::from_static("x-server-source"),
                    HeaderValue::from_static(StaticServerInformation::SERVER.source),
                ),
                (
                    HeaderName::from_static("x-server-environment"),
                    HeaderValue::from_static(StaticServerInformation::SERVER.environment),
                ),
            ]),
            next.run(req).await,
        )
    }
}
