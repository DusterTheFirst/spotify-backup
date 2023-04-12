use axum::{async_trait, extract::FromRequestParts, http::request, response::Redirect};

pub mod spotify;

pub async fn login_github() -> Redirect {
    Redirect::to("/")
}

#[derive(Debug)]
pub struct Authentication {
    pub github: (),
    pub spotify: (),
}

#[async_trait]
impl<S> FromRequestParts<S> for Authentication {
    type Rejection = Redirect;

    async fn from_request_parts(
        parts: &mut request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Err(Redirect::to("/login")) // TODO: implement auth
    }
}
