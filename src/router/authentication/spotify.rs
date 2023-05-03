use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use rspotify::{
    prelude::{Id, OAuthClient},
    scopes, AuthCodeSpotify,
};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, error_span, trace};

use crate::{
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
        user_id: user_id.id().to_string(),
        created_at: OffsetDateTime::now_utc(),
    }
}

pub async fn login(
    State(AppState {
        spotify, database, ..
    }): State<AppState>,
    user_session: Option<UserSession>,
    query: Option<Query<SpotifyAuthCodeResponse>>,
) -> Result<Either<(UserSession, Redirect), Redirect>, InternalServerError> {
    let required_scopes = scopes!("playlist-read-private", "user-library-read");

    let auth = AuthCodeSpotify::new(
        spotify.credentials.clone(),
        rspotify::OAuth {
            redirect_uri: spotify.redirect_uri.to_string(),
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
                    .login_user_by_spotify(
                        user_session.map(|s| s.id),
                        from_rspotify(token, user.id.clone()),
                    )
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
