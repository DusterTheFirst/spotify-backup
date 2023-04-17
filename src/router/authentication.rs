use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::request,
    response::Redirect,
    RequestPartsExt,
};
use axum_extra::{either::Either, extract::CookieJar};
use color_eyre::eyre::Context;

use crate::{
    database::{Database, UserSessionId},
    pages::ErrorPage,
};

use super::AppState;

pub mod spotify;

pub async fn login_github() -> Redirect {
    Redirect::to("/")
}

#[derive(Debug)]
pub struct Account {
    pub account: entity::account::Model,
}

// FIXME: session pruning
const SESSION_COOKIE: &str = "spotify-backup-session";

#[async_trait]
impl<S> FromRequestParts<S> for Account
where
    Database: FromRef<S>,
    S: Sync,
{
    type Rejection = Either<Redirect, ErrorPage>;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let cookies = parts
            .extract::<CookieJar>()
            .await
            .expect("cookie jar should never fail");

        let database = Database::from_ref(state);

        if let Some(session_uuid) = cookies
            .get(SESSION_COOKIE)
            .and_then(|session| session.value().parse().ok())
        {
            if let Some((session, account)) = database
                .get_user_session(UserSessionId::from_raw(session_uuid))
                .await
                .wrap_err("failed to get user session")
                .map_err(|error| Either::E2(error.into()))?
            {
                return Ok(Account { account });
            } else {
                tracing::trace!(%session_uuid, "user had bad session uuid")
            }
        }

        Err(Either::E1(Redirect::to("/login")))
    }
}
