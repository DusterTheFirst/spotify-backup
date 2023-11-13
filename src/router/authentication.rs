use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request,
    response::{IntoResponse, Redirect, Response},
    RequestPartsExt,
};
use axum_extra::either::Either3;
use rspotify::prelude::OAuthClient;
use sea_orm::prelude::Uuid;
use time::OffsetDateTime;
use tracing::error_span;

use crate::{database::Database, pages::InternalServerError};

use self::{github::GithubAuthentication, spotify::SpotifyAuthentication};

use super::session::{UserSession, UserSessionRejection};

pub mod github;
pub mod spotify;

pub async fn logout(
    State(database): State<Database>,
    session: Option<UserSession>,
) -> Result<Response, InternalServerError> {
    if let Some(session) = session {
        let session = database.logout_current_user(session).await?;

        Ok((session, Redirect::to("/")).into_response())
    } else {
        Ok(Redirect::to("/").into_response())
    }
}

#[derive(Debug)]
pub struct IncompleteUser {
    pub session: entity::user_session::Model,
    pub account: IncompleteAccount,
}

#[derive(Debug)]
pub struct IncompleteAccount {
    pub id: Uuid,
    pub created_at: OffsetDateTime,

    #[doc(hidden)]
    pub github: Option<GithubAuthentication>,
    #[doc(hidden)]
    pub spotify: Option<SpotifyAuthentication>,
}

impl IncompleteAccount {
    pub async fn spotify_user(
        &self,
    ) -> Result<Option<rspotify::model::PrivateUser>, InternalServerError> {
        Ok(if let Some(spotify) = &self.spotify {
            Some(
                InternalServerError::wrap(
                    spotify.as_client().current_user(),
                    error_span!("fetching spotify user", account.id = %self.id),
                )
                .await?,
            )
        } else {
            None
        })
    }

    pub async fn github_user(&self) -> Result<Option<octocrab::models::Author>, InternalServerError> {
        Ok(if let Some(github) = &self.github {
            Some(
                InternalServerError::wrap(
                    github.as_client()?.current().user(),
                    error_span!("fetching github user", account.id = %self.id),
                )
                .await?,
            )
        } else {
            None
        })
    }
}

impl IncompleteUser {
    pub fn is_complete(&self) -> bool {
        self.account.github.is_some() && self.account.spotify.is_some()
    }

    #[allow(clippy::result_large_err)]
    pub fn into_complete(self) -> Result<CompleteUser, IncompleteUser> {
        match (self.account.github, self.account.spotify) {
            (Some(github), Some(spotify)) => Ok(CompleteUser {
                session: self.session,
                account: CompleteAccount {
                    id: self.account.id,
                    created_at: self.account.created_at,

                    spotify,
                    github,
                },
            }),

            (github, spotify) => Err(IncompleteUser {
                session: self.session,
                account: IncompleteAccount {
                    github,
                    spotify,
                    ..self.account
                },
            }),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for IncompleteUser
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either3<Redirect, InternalServerError, UserSessionRejection>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let database = Database::from_ref(state);

        match parts.extract::<UserSession>().await {
            Ok(user_session) => {
                let incomplete_user = database
                    .get_current_user(user_session.id)
                    .await
                    .map_err(Either3::E2)?;

                if let Some(incomplete_user) = incomplete_user {
                    return Ok(incomplete_user);
                }
            }
            Err(UserSessionRejection::NoSessionCookie) => {}
            Err(error) => {
                return Err(Either3::E3(error));
            }
        };

        Err(Either3::E1(Redirect::to("/")))
    }
}

#[derive(Debug)]
pub struct CompleteUser {
    pub session: entity::user_session::Model,
    pub account: CompleteAccount,
}

#[derive(Debug)]
pub struct CompleteAccount {
    pub id: Uuid,
    pub created_at: OffsetDateTime,

    github: GithubAuthentication,
    spotify: SpotifyAuthentication,
}

#[async_trait]
impl<S> FromRequestParts<S> for CompleteUser
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either3<Redirect, InternalServerError, UserSessionRejection>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        IncompleteUser::from_request_parts(parts, state)
            .await
            .and_then(|incomplete| {
                incomplete
                    .into_complete()
                    .map_err(|_incomplete| Either3::E1(Redirect::to("/account")))
            })
    }
}
