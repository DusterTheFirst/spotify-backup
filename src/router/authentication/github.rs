use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use color_eyre::{
    eyre::{eyre, Context},
    Help, SectionExt,
};
use reqwest::header;
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, trace};

use crate::{
    pages::InternalServerError,
    router::{session::UserSession, AppState},
    util::UntaggedResult,
};

#[derive(Debug, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum GithubAuthCodeResponse {
    Success {
        code: String,
    },
    Failure {
        error: String,
        error_description: String,
        error_uri: String,
    },
}

#[derive(Debug, Deserialize)]
struct GithubAccessToken {
    access_token: String,
    scope: String,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct GithubAccessTokenError {
    error: String,
    error_description: String,
    error_uri: String,
}

pub async fn login(
    State(AppState {
        github,
        database,
        reqwest,
        ..
    }): State<AppState>,
    user_session: Option<UserSession>,
    query: Option<Query<GithubAuthCodeResponse>>,
) -> Result<Either<(UserSession, Redirect), Redirect>, InternalServerError> {
    if let Some(Query(response)) = query {
        match response {
            GithubAuthCodeResponse::Failure {
                error,
                error_description,
                error_uri,
            } => {
                debug!(
                    ?error,
                    ?error_description,
                    ?error_uri,
                    "failed github oauth"
                );

                Err(eyre!("github authentication did not succeed: {error} {error_description} {error_uri}").into())
            }
            GithubAuthCodeResponse::Success { code } => {
                trace!("succeeded github oauth");

                // FIXME: mess, use octocrab?
                let response = reqwest
                    .get("https://github.com/login/oauth/access_token")
                    .query(&[
                        ("client_id", github.client_id),
                        ("client_secret", github.client_secret),
                        ("redirect_uri", github.redirect_uri.to_string()),
                        ("code", code),
                    ])
                    .header(header::ACCEPT, "application/json")
                    .header("X-GitHub-Api-Version", "2022-11-28")
                    .send()
                    .await
                    .wrap_err("exchanging authorization code (request)")?
                    .error_for_status()
                    .wrap_err("exchanging authorization code (status)")?;

                dbg!(response.headers().get(header::CONTENT_TYPE));

                // TODO: manual serde
                let token_json = response
                    .text()
                    .await
                    .wrap_err("receiving github access_token response")?;

                let oauth = serde_json::from_str::<
                    UntaggedResult<GithubAccessToken, GithubAccessTokenError>,
                >(&token_json)
                .wrap_err("deserializing github access_token response")
                .with_section(|| token_json.clone().header("JSON"))?
                .0
                .map_err(|error| {
                    eyre!("Github API endpoint returned an error")
                        .with_section(|| error.error.header("API Error"))
                        .with_section(|| error.error_description.header("Error Description"))
                        .with_section(|| error.error_uri.header("Error Uri"))
                })?;

                if oauth.scope.is_empty() {
                    return Err(eyre!("github oauth scopes is not empty")
                        // FIXME: replace with tracing spans?
                        .with_section(|| token_json.clone().header("JSON"))
                        .with_section(|| format!("{:?}", oauth.scope).header("scope"))
                        .into());
                }
                if !oauth.token_type.eq_ignore_ascii_case("bearer") {
                    return Err(eyre!("github oauth token type is not bearer")
                        .with_section(|| token_json.clone().header("JSON"))
                        .with_section(|| format!("{:?}", oauth.token_type).header("token_type"))
                        .into());
                }

                let token_created_at = OffsetDateTime::now_utc();

                let client = octocrab::OctocrabBuilder::new()
                    .oauth(octocrab::auth::OAuth {
                        access_token: oauth.access_token.into(),
                        token_type: "bearer".into(),
                        scope: vec![],
                    })
                    .build()
                    .wrap_err("building octocrab client")?;

                let user = client
                    .current()
                    .user()
                    .await
                    .wrap_err("getting current user")?;

                dbg!(&user);

                let new_session = database
                    .login_user_by_github(
                        user_session.map(|s| s.id),
                        entity::github_auth::Model {
                            // Postgres does not have u64 :(
                            user_id: user.id.0.to_string(),
                            access_token: "".to_string(),
                            created_at: token_created_at,
                        },
                    )
                    .await
                    .wrap_err("logging in github account")?;

                Ok(Either::E1((
                    UserSession { id: new_session },
                    Redirect::to("/account"),
                )))
            }
        }
    } else {
        todo!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
            github.client_id,
            github.redirect_uri
        )
        // Ok(Either::E2(Redirect::to(&format!(
        //     "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
        //     github.client_id, github.redirect_uri
        // ))))
    }
}
