use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, Query, State},
    http::request,
    response::{Html, IntoResponse, Redirect, Response},
    RequestPartsExt,
};
use axum_extra::either::Either3;
use rspotify::prelude::OAuthClient;
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::error_span;

use crate::{
    database::{id::AccountId, Database},
    pages::InternalServerError,
};

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

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    decree: String,
}

const DECREE: &str = "I solemnly swear that I am deleting my account";

pub async fn delete(
    State(database): State<Database>,
    user: Option<User>,
    query: Option<Query<DeleteQuery>>,
) -> Result<Response, InternalServerError> {
    // FIXME: make this flow cleaner
    if let Some(user) = user {
        if let Some(query) = query {
            if query.decree == DECREE {
                let session = database.delete_current_user(user).await?;

                Ok((session, Redirect::to("/")).into_response())
            } else {
                Ok(("Invalid decree, your account is not deleted").into_response())
            }
        } else {
            Ok(Html(format!("<form action=\"/logout/delete\" method=\"get\"><label>Please type the following phrase: <pre>{DECREE}</pre> <input type=\"text\" name=\"decree\" /></label><button type=\"submit\">Delete my account</button><a href=\"/account\">Go back</a></form>")).into_response())
        }
    } else {
        Ok(Redirect::to("/").into_response())
    }
}

#[derive(Debug)]
pub struct User {
    pub session: entity::user_session::Model,
    pub account: Account,
}

#[derive(Debug)]
pub struct Account {
    pub id: AccountId,
    pub created_at: OffsetDateTime,

    pub spotify: SpotifyAuthentication,
    pub github: Option<GithubAuthentication>,
}

impl Account {
    #[tracing::instrument(skip(self), fields(account.id = ?self.id))]
    pub async fn spotify_user(&self) -> Result<rspotify::model::PrivateUser, InternalServerError> {
        InternalServerError::wrap_in_current_span(self.spotify.as_client().current_user()).await
    }

    #[tracing::instrument(skip(self), fields(account.id = ?self.id))]
    pub async fn github_user(
        &self,
    ) -> Result<Option<octocrab::models::Author>, InternalServerError> {
        Ok(if let Some(github) = &self.github {
            Some(
                InternalServerError::wrap_in_current_span(github.as_client()?.current().user())
                    .await?,
            )
        } else {
            None
        })
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for User
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
                let user = database
                    .get_current_user(user_session.id)
                    .await
                    .map_err(Either3::E2)?;

                if let Some(user) = user {
                    return Ok(user);
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
