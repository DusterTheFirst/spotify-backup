use std::convert::Infallible;

use axum::{async_trait, extract::FromRequestParts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use color_eyre::eyre::Context;
use tracing::debug;

use crate::database::{AccountId, Database, UserSessionId};

const SESSION_COOKIE: &str = "spotify-backup-session";

pub async fn create_user_session(
    cookies: CookieJar,
    database: Database,
    account: entity::account::Model,
) -> Result<(CookieJar, UserSession), color_eyre::eyre::Report> {
    let session = database
        .create_user_session(AccountId::from_raw(account.id))
        .await
        .wrap_err("failed to create user session")?;

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

    Ok((new_cookie, UserSession { session, account }))
}

#[derive(Debug, Clone)]
pub struct UserSession {
    pub session: entity::user_session::Model,
    pub account: entity::account::Model,
}
