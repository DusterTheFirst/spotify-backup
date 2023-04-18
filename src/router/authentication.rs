use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request,
    response::Redirect,
    RequestPartsExt,
};
use axum_extra::either::Either;
use color_eyre::{eyre::Context, Report};

use crate::{database::Database, pages::ErrorPage};

use super::session::{UserSessionId, UserSessionIdRejection};

pub mod spotify;

pub async fn login_github() -> Redirect {
    Redirect::to("/")
}

#[derive(Debug)]
pub struct User {
    pub session: entity::user_session::Model,
    pub account: entity::account::Model,
}

#[async_trait]
impl<S> FromRequestParts<S> for User
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either<Redirect, ErrorPage>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let database = Database::from_ref(state);

        match parts.extract::<UserSessionId>().await {
            Ok(user_session) => {
                if let Some((session, account)) = database
                    .get_user_session(user_session)
                    .await
                    .wrap_err("failed to get user session")
                    .map_err(|error| Either::E2(error.into()))?
                {
                    return Ok(User { session, account });
                }
            }
            Err(UserSessionIdRejection::BadSessionCookie(error)) => {
                // TODO: probably should be 400 not 500
                return Err(Either::E2(ErrorPage::from(
                    Report::new(error).wrap_err("unable to extract user session id"),
                )));
            }
            Err(UserSessionIdRejection::NoSessionCookie) => {}
        };

        Err(Either::E1(Redirect::to("/login")))
    }
}
