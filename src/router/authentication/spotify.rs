use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use color_eyre::eyre::Context;
use rspotify::{
    prelude::{Id, OAuthClient},
    scopes, AuthCodeSpotify,
};
use serde::Deserialize;
use thiserror::Error;
use time::OffsetDateTime;
use tracing::{debug, info, trace};

use super::super::middleware::request_metadata::RequestMetadata;
use crate::{
    database::{Database, SpotifyId},
    pages, SpotifyEnvironment,
};

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

pub fn from_rspotify(
    token: rspotify::Token,
    user_id: rspotify::model::UserId,
) -> entity::spotify_auth::Model {
    entity::spotify_auth::Model {
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
        scopes: token.scopes.iter().collect(),
        user_id: user_id.id().to_string(),
        created: OffsetDateTime::now_utc(),
    }
}

pub async fn login(
    State((spotify, database)): State<(SpotifyEnvironment, Database)>,
    session: UserSession,
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

                let spotify_id = SpotifyId::from_raw(user.id);

                dbg!(&spotify_id);

                let created = database
                    .update_user_authentication(spotify_id, token)
                    .await
                    .wrap_err("failed to update into database")
                    .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

                dbg!(&created);

                let account = database.get_or_create_account_by_spotify(created.id).await;

                dbg!(account);

                return Ok(Redirect::to("/"));
            }
        }
    }

    // TODO: sessions, merge logged-in path with logging in path
    // let authentication: Option<SpotifyToken> = database
    //     .get_user_authentication("dusterthefirst")
    //     .await
    //     .wrap_err("failed to select into database")
    //     .map_err(|error| pages::dyn_error(error.as_ref(), &request_metadata))?;

    // if let Some(authentication) = authentication {
    //     // TODO: verify the credentials
    //     info!("user attempted to re-login with existing credentials");

    //     return Ok(Redirect::to("/already/logged/in"));
    // }

    // TODO: distinguish return users?
    let auth_url = auth
        .get_authorize_url(true)
        .expect("authorization url should be valid");

    Ok(Redirect::to(&auth_url))
}
