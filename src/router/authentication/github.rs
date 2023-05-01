use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use reqwest::header;
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, error_span, trace, Instrument};

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

                Err(InternalServerError::new(error_span!(
                    "github authentication failure",
                    error,
                    error_description,
                    error_uri,
                )))
            }
            GithubAuthCodeResponse::Success { code } => {
                trace!("succeeded github oauth");

                // FIXME: mess, use octocrab?
                let response = async {
                    reqwest
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
                        .await?
                        .error_for_status()
                }
                .instrument(error_span!("exchanging authorization code").or_current())
                .await?;

                dbg!(response.headers().get(header::CONTENT_TYPE));

                // TODO: manual serde
                let token_json = response
                    .text()
                    .instrument(error_span!("receiving github access_token response"))
                    .await?;

                let oauth = error_span!(
                    "deserializing github access_token response",
                    json = token_json
                )
                .in_scope(|| {
                    let deserialized_json: UntaggedResult<
                        GithubAccessToken,
                        GithubAccessTokenError,
                    > = serde_json::from_str(&token_json)?;

                    let oauth = deserialized_json.0.map_err(
                        |GithubAccessTokenError {
                             error,
                             error_description,
                             error_uri,
                         }| {
                            InternalServerError::new(error_span!(
                                "github endpoint returned error",
                                error,
                                error_description,
                                error_uri,
                            ))
                        },
                    )?;

                    if !oauth.scope.is_empty() {
                        return Err(InternalServerError::new(error_span!(
                            "github oauth scopes is not empty",
                            oauth.scope,
                        )));
                    }
                    if !oauth.token_type.eq_ignore_ascii_case("bearer") {
                        return Err(InternalServerError::new(error_span!(
                            "github oauth token type is not bearer",
                            oauth.token_type,
                        )));
                    }

                    Ok(oauth)
                })?;

                let token_created_at = OffsetDateTime::now_utc();

                // TODO: macro or smthn for error_span, in_scope, in_current_span
                let client = error_span!("building octocrab client").in_scope(|| {
                    tracing_error::InstrumentResult::in_current_span(
                        octocrab::OctocrabBuilder::new()
                            .oauth(octocrab::auth::OAuth {
                                access_token: oauth.access_token.into(),
                                token_type: "bearer".into(),
                                scope: vec![],
                            })
                            .build(),
                    )
                })?;

                let user = client
                    .current()
                    .user()
                    .instrument(error_span!("getting current user"))
                    .await?;

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
                    .instrument(error_span!(
                        "logging in github account",
                        user.id = user.id.0
                    ))
                    .await?;

                Ok(Either::E1((
                    UserSession { id: new_session },
                    Redirect::to("/account"),
                )))
            }
        }
    } else {
        // todo!(
        //     "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
        //     github.client_id,
        //     github.redirect_uri
        // )
        Ok(Either::E2(Redirect::to(&format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
            github.client_id, github.redirect_uri
        ))))
    }
}
