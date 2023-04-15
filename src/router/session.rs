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
use sea_orm::IntoActiveModel;
use tracing::{debug, trace, Instrument};

use crate::{
    database::{Database, UserSessionId},
    pages,
};

use super::middleware::request_metadata::RequestMetadata;

// FIXME: do not keep forever, right now polling /health will spam create sessions
pub async fn user_session(
    cookies: CookieJar,
    request_meta: RequestMetadata,
    State(database): State<Database>,
    mut req: Request<Body>,
    next: Next<Body>,
) -> Result<Response<BoxBody>, Response<BoxBody>> {
    const SESSION_COOKIE: &str = "spotify-backup-session";

    if let Some(session_uuid) = cookies
        .get(SESSION_COOKIE)
        .and_then(|session| session.value().parse().ok())
    {
        let session = database
            .get_user_session(UserSessionId::from_raw(session_uuid))
            .await
            .map_err(|error| pages::dyn_error(&error, &request_meta).into_response())?;

        if let Some((session, account)) = session {
            let session_id = session.id;
            trace!(?session_id, "existing user, found session");

            req.extensions_mut()
                .insert(UserSession { session, account });

            let inner = next
            .run(req)
            .instrument(tracing::debug_span!(target: "spotify_backup", "session::existing", ?session_id))
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

    let session_id = session.id;
    debug!(?session_id, "new user, created session");

    let new_cookie = cookies.add(
        Cookie::build(SESSION_COOKIE, session.id.to_string())
            .path("/")
            .same_site(SameSite::Lax)
            .secure(true)
            .http_only(true)
            .finish(),
    );

    req.extensions_mut().insert(UserSession {
        session,
        account: None,
    });

    let inner = next
        .run(req)
        .instrument(tracing::debug_span!(target: "spotify_backup", "session::new", ?session_id))
        .await;

    Ok((new_cookie, inner).into_response())
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub session: entity::user_session::Model,
    pub account: Option<entity::account::Model>,
}

#[async_trait]
impl<S> FromRequestParts<S> for UserSession {
    type Rejection = Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<UserSession>()
            .expect("session middleware should add UserSession extension")
            .clone())
    }
}
