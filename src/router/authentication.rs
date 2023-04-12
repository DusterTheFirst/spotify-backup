use axum::{
    async_trait,
    extract::{FromRequestParts, Query, State},
    http::request,
    response::{IntoResponse, Redirect},
};
use rspotify::{scopes, AuthCodeSpotify};
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, trace};

use crate::{
    pages::{self, Page},
    SpotifyEnvironment,
};

use super::middleware::RequestMetadata;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SpotifyAuthCodeResponse {
    Success { code: String, state: String },
    Failure { error: String, state: String },
}

#[derive(Error, Debug)]
pub enum SpotifyLoginError {
    #[error("spotify authentication did not succeed: {error}")]
    AuthCodeResponseFailure { error: String },
}

pub async fn login_spotify(
    State(spotify): State<SpotifyEnvironment>,
    request_metadata: RequestMetadata,
    query: Option<Query<SpotifyAuthCodeResponse>>,
) -> Result<Redirect, impl IntoResponse> {
    if let Some(Query(response)) = query {
        match response {
            SpotifyAuthCodeResponse::Failure { error, state } => {
                debug!(?error, "failed spotify login");

                return Err(pages::dyn_error(
                    &SpotifyLoginError::AuthCodeResponseFailure { error },
                    request_metadata,
                ));
            }
            SpotifyAuthCodeResponse::Success { code, state } => {
                trace!("succeeded spotify login");

                return Ok(Redirect::to("/"));
            }
        }
    }

    let auth = AuthCodeSpotify::new(
        spotify.credentials,
        rspotify::OAuth {
            redirect_uri: spotify.redirect_uri.to_string(),
            scopes: scopes!("playlist-read-private", "user-library-read"),
            ..Default::default()
        },
    );

    // TODO: distinguish return users?
    let auth_url = auth
        .get_authorize_url(true)
        .expect("authorization url should be valid");

    Ok(Redirect::to(&auth_url))
}

pub async fn login_github() -> Redirect {
    Redirect::to("/")
}

#[derive(Debug)]
pub struct Authentication {
    pub github: (),
    pub spotify: (),
}

#[async_trait]
impl<S> FromRequestParts<S> for Authentication {
    type Rejection = Redirect;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Err(Redirect::to("/login")) // TODO: implement auth
    }
}
