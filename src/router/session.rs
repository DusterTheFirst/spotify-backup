use std::{convert::Infallible, str::FromStr};

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::StatusCode,
    response::{IntoResponse, IntoResponseParts},
    RequestPartsExt,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, Expiration, SameSite};
use sea_orm::prelude::Uuid;
use time::{Duration, OffsetDateTime};
use tracing::debug;

use crate::database::id::UserSessionId;

const SESSION_COOKIE: &str = "spotify-backup-session";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSession {
    pub id: UserSessionId,
}

impl UserSession {
    pub const fn remove() -> UserSession {
        UserSession {
            id: UserSessionId::from_raw(Uuid::nil()),
        }
    }
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
            // FIXME: delete bad session cookes from user agent?
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
        let uuid = self.id.into_uuid();

        let cookie: Cookie = Cookie::build(SESSION_COOKIE, uuid.to_string())
            .path("/")
            .same_site(SameSite::Lax)
            .secure(true)
            .http_only(true)
            .expires(if uuid.is_nil() {
                // If session id is nil, invalidate session
                Expiration::DateTime(OffsetDateTime::UNIX_EPOCH)
            } else {
                // Else persist this cookie for the whole session
                Expiration::Session
            })
            .finish();

        CookieJar::new().add(cookie).into_response_parts(res)
    }
}
