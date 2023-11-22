use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use rspotify::{
    model::{PrivateUser, UserId},
    prelude::OAuthClient,
    scopes, AuthCodeSpotify, Token,
};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, error_span, trace};

use crate::{
    environment::SPOTIFY_ENVIRONMENT,
    internal_server_error,
    pages::InternalServerError,
    router::{session::UserSession, AppState},
};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum SpotifyAuthCodeResponse {
    Success { code: String, state: String },
    Failure { error: String, state: String },
}

pub async fn login(
    State(AppState { database, .. }): State<AppState>,
    user_session: Option<UserSession>,
    query: Option<Query<SpotifyAuthCodeResponse>>,
) -> Result<Either<(UserSession, Redirect), Redirect>, InternalServerError> {
    let required_scopes = scopes!("playlist-read-private", "user-library-read");

    // FIXME: unify?
    let auth = AuthCodeSpotify::new(
        SPOTIFY_ENVIRONMENT.credentials.clone(),
        rspotify::OAuth {
            redirect_uri: SPOTIFY_ENVIRONMENT.redirect_uri.to_string(),
            scopes: required_scopes.clone(),
            ..Default::default()
        },
    );

    if let Some(Query(response)) = query {
        match response {
            SpotifyAuthCodeResponse::Failure { error, state } => {
                debug!(?error, "failed spotify oauth");

                Err(internal_server_error!(
                    "spotify authentication did not succeed",
                    error
                ))
            }
            SpotifyAuthCodeResponse::Success { code, state } => {
                trace!("succeeded spotify oauth");

                InternalServerError::wrap(
                    auth.request_token(&code),
                    error_span!("requesting access token"),
                )
                .await?;

                let token = auth
                    .token
                    .lock()
                    .await
                    .expect("spotify client token mutex should not be poisoned")
                    .clone()
                    .expect("spotify client token should exist");

                if !token.scopes.is_superset(&required_scopes) {
                    return Err(internal_server_error!(
                        "spotify scopes did not match required scopes",
                        ?required_scopes,
                        ?token.scopes
                    ));
                }

                // FIXME: 403 when user is outside of allowlist
                // https://developer.spotify.com/documentation/web-api/concepts/quota-modes
                let user = InternalServerError::wrap(
                    auth.current_user(),
                    error_span!("getting current user"),
                )
                .await?;

                let new_session = database
                    .login_user(user_session, SpotifyAuthentication::create(token, user))
                    .await?;

                Ok(Either::E1((
                    UserSession { id: new_session },
                    Redirect::to("/account"),
                )))
            }
        }
    } else {
        let auth_url = auth
            .get_authorize_url(false)
            .expect("authorization url should be valid");

        Ok(Either::E2(Redirect::to(&auth_url)))
    }
}

#[derive(Debug)]
pub struct SpotifyAuthentication {
    access_token: SecretString,
    refresh_token: SecretString,

    pub user_id: UserId<'static>,
    pub expires_at: OffsetDateTime,
    pub created_at: OffsetDateTime,
}

impl SpotifyAuthentication {
    fn create(token: Token, user: PrivateUser) -> Self {
        Self {
            access_token: SecretString::new(token.access_token),
            refresh_token: token
                .refresh_token
                .map(SecretString::new)
                .expect("refresh token should exist"),
            user_id: user.id,
            expires_at: time::OffsetDateTime::from_unix_timestamp(
                token
                    .expires_at
                    .expect("rspotify token should have a calculated expiration date")
                    .timestamp(),
            )
            .expect("UNIX timestamp returned from chrono should be valid"),
            created_at: OffsetDateTime::now_utc(),
        }
    }

    pub fn as_client(&self) -> AuthCodeSpotify {
        AuthCodeSpotify::from_token(Token {
            access_token: self.access_token.expose_secret().clone(),
            expires_in: chrono::Duration::seconds(0),
            expires_at: Some(chrono::DateTime::from_naive_utc_and_offset(
                chrono::NaiveDateTime::from_timestamp_millis(
                    self.expires_at.unix_timestamp() * 1000,
                )
                .expect("UNIX timestamp returned from time should be valid"),
                chrono::Utc,
            )),
            refresh_token: Some(self.refresh_token.expose_secret().clone()),
            scopes: scopes!("playlist-read-private", "user-library-read"),
        })
    }

    pub fn into_model(self) -> entity::spotify_auth::Model {
        entity::spotify_auth::Model {
            user_id: self.user_id.to_string(),
            access_token: self.access_token.expose_secret().clone(),
            expires_at: self.expires_at,
            refresh_token: self.refresh_token.expose_secret().clone(),
            created_at: self.created_at,
        }
    }

    pub fn from_model(model: entity::spotify_auth::Model) -> Self {
        Self {
            access_token: SecretString::new(model.access_token),
            refresh_token: SecretString::new(model.refresh_token),
            user_id: UserId::from_uri(&model.user_id)
                .expect("user id should be valid")
                .into_static(),
            expires_at: model.expires_at,
            created_at: model.created_at,
        }
    }
}
