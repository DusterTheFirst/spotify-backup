use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use color_eyre::eyre::{eyre, Context};
use rspotify::{
    prelude::{Id, OAuthClient},
    scopes, AuthCodeSpotify,
};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, trace};

use crate::{
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
        scopes: token.scopes.into_iter().collect(),
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
    let auth = AuthCodeSpotify::new(
        spotify.credentials.clone(),
        rspotify::OAuth {
            redirect_uri: spotify.redirect_uri.to_string(),
            scopes: scopes!("playlist-read-private", "user-library-read"),
            ..Default::default()
        },
    );

    if let Some(Query(response)) = query {
        match response {
            SpotifyAuthCodeResponse::Failure { error, state } => {
                debug!(?error, "failed spotify oauth");

                Err(eyre!("spotify authentication did not succeed: {error}").into())
            }
            SpotifyAuthCodeResponse::Success { code, state } => {
                trace!("succeeded spotify oauth");

                auth.request_token(&code)
                    .await
                    .wrap_err("failed to request access token")?;

                // FIXME: 403 when user is outside of allowlist
                // https://developer.spotify.com/documentation/web-api/concepts/quota-modes
                let user = auth
                    .current_user()
                    .await
                    .wrap_err("unable to get current user")?;

                let token = auth
                    .token
                    .lock()
                    .await
                    .expect("spotify client token mutex should not be poisoned");

                let new_session = database
                    .login_user_by_spotify(
                        user_session.map(|s| s.id),
                        from_rspotify(
                            token.clone().expect("spotify client token should exist"),
                            user.id.clone(),
                        ),
                    )
                    .await
                    .wrap_err("failed to login to spotify account")?;

                Ok(Either::E1((
                    UserSession { id: new_session },
                    Redirect::to("/account"),
                )))
            }
        }
    } else {
        let auth_url = auth
            .get_authorize_url(true)
            .expect("authorization url should be valid");

        Ok(Either::E2(Redirect::to(&auth_url)))
    }
}
