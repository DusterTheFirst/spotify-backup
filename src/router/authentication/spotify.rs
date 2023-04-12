use std::collections::HashSet;

use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use color_eyre::eyre::Context;
use rspotify::{
    prelude::{Id, OAuthClient},
    scopes, AuthCodeSpotify,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, info, trace};

use super::super::middleware::RequestMetadata;
use crate::{database::Database, pages, SpotifyEnvironment};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SpotifyAuthCodeResponse {
    Success { code: String, state: String },
    Failure { error: String, state: String },
}

#[derive(Error, Debug)]
pub enum SpotifyLoginError {
    #[error("spotify authentication did not succeed: {error}")]
    AuthCodeRedirectFailure { error: String },
}

#[derive(Debug, Serialize, Deserialize)]
struct SpotifyToken {
    access_token: String,
    #[serde(with = "time::serde::timestamp")]
    expires_at: time::OffsetDateTime,
    refresh_token: String,
    scopes: HashSet<String>,
}

impl SpotifyToken {
    pub fn from_rspotify(token: rspotify::Token) -> Self {
        Self {
            access_token: token.access_token,
            expires_at: time::OffsetDateTime::from_unix_timestamp(
                token
                    .expires_at
                    .expect("rspotify token should have a calculated expiration date")
                    .timestamp(),
            )
            .expect("UNIX timestamp returned from chrono should be valid"),
            refresh_token: token
                .refresh_token
                .expect("rspotify token should contain refresh token"),
            scopes: token.scopes,
        }
    }
}

pub async fn login(
    State((spotify, database)): State<(SpotifyEnvironment, Database)>,
    request_metadata: RequestMetadata,
    query: Option<Query<SpotifyAuthCodeResponse>>,
) -> Result<Redirect, impl IntoResponse> {
    let auth = AuthCodeSpotify::new(
        spotify.credentials,
        rspotify::OAuth {
            redirect_uri: spotify.redirect_uri.to_string(),
            scopes: scopes!("playlist-read-private", "user-library-read"),
            ..Default::default()
        },
    );

    if let Some(Query(response)) = query {
        match response {
            SpotifyAuthCodeResponse::Failure { error, state } => {
                debug!(?error, "failed spotify login");

                return Err(pages::dyn_error(
                    &SpotifyLoginError::AuthCodeRedirectFailure { error },
                    &request_metadata,
                ));
            }
            SpotifyAuthCodeResponse::Success { code, state } => {
                trace!("succeeded spotify login");

                auth.request_token(&code)
                    .await
                    .wrap_err("failed to request access token")
                    .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

                let user = auth
                    .current_user()
                    .await
                    .wrap_err("unable to get current user")
                    .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

                let token = auth
                    .token
                    .lock()
                    .await
                    .expect("spotify client token mutex should not be poisoned");

                let token = SpotifyToken::from_rspotify(
                    token.clone().expect("spotify client token should exist"),
                );

                let created: SpotifyToken = database
                    .update(("spotify_authentication", user.id.id()))
                    .content(token)
                    .await
                    .wrap_err("failed to update into database")
                    .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

                dbg!(created);

                return Ok(Redirect::to("/"));
            }
        }
    }

    // TODO: sessions
    let authentication: Option<SpotifyToken> = database
        .select(("spotify_authentication", "dusterthefirst"))
        .await
        .wrap_err("failed to select into database")
        .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

    if let Some(authentication) = authentication {
        // TODO: verify the credentials
        info!(
            ?authentication,
            "user attempted to re-login with existing credentials"
        );

        return Ok(Redirect::to("/already/logged/in"));
    }

    // TODO: distinguish return users?
    let auth_url = auth
        .get_authorize_url(true)
        .expect("authorization url should be valid");

    Ok(Redirect::to(&auth_url))
}
