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

use crate::database::id::UserSessionId;

const SESSION_COOKIE: &str = "spotify-backup-session";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSession {
    pub id: UserSessionId,
}

#[derive(Debug)]
pub enum UserSessionRejection {
    NoSessionCookie,
    BadSessionCookie(<Uuid as FromStr>::Err),
}

impl IntoResponse for UserSessionRejection {
    fn into_response(self) -> axum::response::Response {
        match self {
            UserSessionRejection::NoSessionCookie => StatusCode::UNAUTHORIZED.into_response(),
            UserSessionRejection::BadSessionCookie(error) => {
                debug!(?error, "user has bad session cookie");

                StatusCode::BAD_REQUEST.into_response()
            }
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSession {
    type Rejection = UserSessionRejection;

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
                .map_err(UserSessionRejection::BadSessionCookie)?;

            Ok(UserSession {
                id: UserSessionId::from_raw(uuid),
            })
        } else {
            Err(UserSessionRejection::NoSessionCookie)
        }
    }
}

impl IntoResponseParts for UserSession {
    type Error = Infallible;

    fn into_response_parts(
        self,
        res: axum::response::ResponseParts,
    ) -> Result<axum::response::ResponseParts, Self::Error> {
        CookieJar::new()
            .add(
                Cookie::build(SESSION_COOKIE, self.id.into_uuid().to_string())
                    .path("/")
                    .same_site(SameSite::Lax)
                    .secure(true)
                    .http_only(true)
                    .finish(),
            )
            .into_response_parts(res)
    }
}
