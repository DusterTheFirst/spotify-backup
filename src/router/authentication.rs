use axum::{async_trait, extract::FromRequestParts, http::request, response::Redirect};

use super::session::UserSession;

pub mod spotify;

pub async fn login_github() -> Redirect {
    Redirect::to("/")
}

#[derive(Debug)]
pub struct Account {
    pub account: entity::account::Model,
}

#[async_trait]
impl<S> FromRequestParts<S> for Account {
    type Rejection = Redirect;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let session = parts
            .extensions
            .get::<UserSession>()
            .expect("session middleware should add UserSession extension");

        match session.account.clone() {
            Some(account) => Ok(Account { account }),
            None => Err(Redirect::to("/login")),
        }
    }
}
