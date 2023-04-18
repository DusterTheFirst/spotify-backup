use std::{convert::Infallible, str::FromStr};

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::StatusCode,
    response::{IntoResponse, IntoResponseParts},
    RequestPartsExt,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use sea_orm::prelude::Uuid;
use tracing::debug;

const SESSION_COOKIE: &str = "spotify-backup-session";

#[derive(Debug)]
pub struct UserSessionId(Uuid);

impl UserSessionId {
    pub fn from_user_session(session: entity::user_session::Model) -> Self {
        Self(session.id)
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

#[derive(Debug)]
pub enum UserSessionIdRejection {
    NoSessionCookie,
    BadSessionCookie(<Uuid as FromStr>::Err),
}

impl IntoResponse for UserSessionIdRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            UserSessionIdRejection::NoSessionCookie => StatusCode::UNAUTHORIZED.into_response(),
            UserSessionIdRejection::BadSessionCookie(error) => {
                debug!(?error, "user has bad session cookie");

                StatusCode::BAD_REQUEST.into_response()
            }
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSessionId {
    type Rejection = UserSessionIdRejection;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let cookies = parts
            .extract::<CookieJar>()
            .await
            .expect("cookie jar should never fail");

        if let Some(cookie) = cookies.get(SESSION_COOKIE) {
            let uuid = cookie
                .value()
                .parse()
                .map_err(UserSessionIdRejection::BadSessionCookie)?;

            Ok(UserSessionId(uuid))
        } else {
            Err(UserSessionIdRejection::NoSessionCookie)
        }
    }
}

impl IntoResponseParts for UserSessionId {
    type Error = Infallible;

    fn into_response_parts(
        self,
        res: axum::response::ResponseParts,
    ) -> Result<axum::response::ResponseParts, Self::Error> {
        CookieJar::new()
            .add(
                Cookie::build(SESSION_COOKIE, self.0.to_string())
                    .path("/")
                    .same_site(SameSite::Lax)
                    .secure(true)
                    .http_only(true)
                    .finish(),
            )
            .into_response_parts(res)
    }
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub session: entity::user_session::Model,
    pub account: entity::account::Model,
}
