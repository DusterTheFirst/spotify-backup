use axum::{
    extract::{Query, State},
    response::Redirect,
};
use serde::Deserialize;

use crate::{router::session::UserSession, GithubEnvironment};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GithubAuthCodeResponse {
    Success {
        code: String,
        state: String,
    },
    Failure {
        error: String,
        error_description: String,
        error_uri: String,
        state: String,
    },
}

pub async fn login(
    State(state): State<GithubEnvironment>,
    user_session: Option<UserSession>,
    query: Option<Query<GithubAuthCodeResponse>>,
) -> Redirect {
    let client_id = state.oauth_credentials.id;
    let redirect_uri = state.redirect_uri;

    if let Some(Query(response)) = query {
        todo!();
    }

    Redirect::to(&format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}"
    ))
}
