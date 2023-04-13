use std::convert::Infallible;

use axum::{extract::FromRequestParts, http::request, Extension, RequestPartsExt};
use git_version::git_version;
use once_cell::sync::Lazy;
use serde::Serialize;
use tower_http::request_id::RequestId;

#[derive(Debug)]
pub struct RequestMetadata {
    /// The request ID
    pub request_id: String,
    /// Static build-time server information
    pub server: StaticServerInformation,
    /// Server region
    pub region: &'static str,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for RequestMetadata
where
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Extension(request_id) = parts
            .extract::<Extension<RequestId>>()
            .await
            .expect("RequestId extension should always exist on server");

        static REGION: Lazy<String> = Lazy::new(|| {
            std::env::var("FLY_REGION")
                .ok()
                .unwrap_or_else(|| "local".to_string())
        });

        return Ok(RequestMetadata {
            request_id: request_id
                .header_value()
                .to_str()
                .expect("RequestId should be valid utf-8")
                .to_string(),
            server: StaticServerInformation::new(),
            region: Lazy::force(&REGION).as_str(),
        });
    }
}

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
    pub const fn new() -> StaticServerInformation {
        StaticServerInformation {
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
        }
    }
}
