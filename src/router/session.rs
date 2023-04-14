use std::convert::Infallible;

use axum::{
    async_trait,
    body::{Body, BoxBody},
    extract::{FromRequestParts, State},
    http::{Request, Response},
    middleware::Next,
    response::IntoResponse,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::{debug, trace, Instrument};

use crate::{
    database::{Database, SpotifyId},
    pages,
};

use super::middleware::request_metadata::RequestMetadata;

pub async fn user_session(
    cookies: CookieJar,
    request_meta: RequestMetadata,
    State(database): State<Database>,
    mut req: Request<Body>,
    next: Next<Body>,
) -> Result<Response<BoxBody>, Response<BoxBody>> {
    const SESSION_COOKIE: &str = "spotify-backup-session";

    if let Some(session) = cookies.get(SESSION_COOKIE) {
        let session = database
            .get_user_session(UserSessionId::from_raw(session.value()))
            .await
            .map_err(|error| pages::dyn_error(&error, &request_meta).into_response())?;

        if let Some(session) = session {
            trace!(?session.id, "existing user, found session");

            req.extensions_mut().insert(session.data);

            let inner = next
            .run(req)
            .instrument(tracing::debug_span!(target: "spotify_backup", "session::existing", ?session.id))
            .await;

            return Ok(inner);
        } else {
            debug!("existing user, bad session");
        }
    }

    let session = database
        .create_user_session()
        .await
        .map_err(|error| pages::dyn_error(&error, &request_meta).into_response())?;

    debug!(?session.id, "new user, created session");

    let new_cookie = cookies.add(
        Cookie::build(SESSION_COOKIE, session.id.to_raw())
            .path("/")
            .same_site(SameSite::Lax)
            .secure(true)
            .http_only(true)
            .finish(),
    );

    req.extensions_mut().insert(session.data);

    let inner = next
        .run(req)
        .instrument(tracing::debug_span!(target: "spotify_backup", "session::new", ?session.id))
        .await;

    Ok((new_cookie, inner).into_response())
}

impl UserSession {
    pub fn new() -> Self {
        UserSession {
            account: None,
            last_seen: OffsetDateTime::now_utc(),
        }
    }

    pub fn last_seen(&self) -> OffsetDateTime {
        self.last_seen
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSession {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<UserSession>()
            .expect("session middleware should add UserSession extension")
            .clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub spotify: SpotifyId,
    pub github: GithubId,
}
