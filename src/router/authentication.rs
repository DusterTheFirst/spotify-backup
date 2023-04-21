use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::request,
    response::{IntoResponse, Redirect, Response},
    RequestPartsExt,
};
use axum_extra::either::Either3;
use color_eyre::eyre::Context;
use sea_orm::prelude::Uuid;
use time::OffsetDateTime;

use crate::{
    database::{id::SpotifyUserId, Database},
    pages::ErrorPage,
};

use super::session::{UserSession, UserSessionRejection};

pub mod github;
pub mod spotify;

pub async fn logout(
    State(database): State<Database>,
    session: Option<UserSession>,
) -> Result<Response, ErrorPage> {
    if let Some(session) = session {
        let session = database.logout_user_session(session).await?;

        Ok((session, Redirect::to("/")).into_response())
    } else {
        Ok(Redirect::to("/").into_response())
    }
}

#[derive(Debug)]
pub struct IncompleteUser {
    pub session: entity::user_session::Model,
    pub account: entity::account::Model,
}

impl IncompleteUser {
    pub fn is_complete(&self) -> bool {
        self.account.github.is_some() && self.account.spotify.is_some()
    }

    #[allow(clippy::result_large_err)]
    pub fn into_complete(mut self) -> Result<CompleteUser, IncompleteUser> {
        // Do not move out of self until we are sure we are converting to a complete user
        let github = self.account.github.as_mut();
        let spotify = self.account.spotify.as_mut();

        if let Some((github, spotify)) = github.zip(spotify) {
            Ok(CompleteUser {
                session: self.session,
                account: CompleteAccount {
                    id: self.account.id,
                    spotify: SpotifyUserId::from_raw(std::mem::take(spotify)),
                    github: (), // FIXME: TODO:
                    created: self.account.created,
                },
            })
        } else {
            Err(self)
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for IncompleteUser
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either3<Redirect, ErrorPage, UserSessionRejection>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let database = Database::from_ref(state);

        match parts.extract::<UserSession>().await {
            Ok(user_session) => {
                if let Some((session, account)) = database
                    .get_user_session(user_session.id)
                    .await
                    .wrap_err("failed to get user session")
                    .map_err(|error| Either3::E2(error.into()))?
                {
                    return Ok(IncompleteUser { session, account });
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
    pub spotify: SpotifyUserId,
    pub github: (), // GithubUserId
    pub created: OffsetDateTime,
}

#[async_trait]
impl<S> FromRequestParts<S> for CompleteUser
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either3<Redirect, ErrorPage, UserSessionRejection>;

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
