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
    internal_server_error,
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

                Err(internal_server_error!(
                    "github authentication failure",
                    error,
                    error_description,
                    error_uri,
                ))
            }
            GithubAuthCodeResponse::Success { code } => {
                trace!("succeeded github oauth");

                // FIXME: mess, use octocrab?
                let response = InternalServerError::wrap(
                    async {
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
                    },
                    error_span!("exchanging authorization code").or_current(),
                )
                .await?;

                // TODO: manual serde
                let token_json = InternalServerError::wrap(
                    response.text(),
                    error_span!("receiving github access_token response"),
                )
                .await?;

                let oauth = error_span!(
                    "deserializing github access_token response",
                    json = token_json
                )
                .in_scope(|| {
                    let deserialized_json: UntaggedResult<
                        GithubAccessToken,
                        GithubAccessTokenError,
                    > = serde_json::from_str(&token_json)
                        .map_err(InternalServerError::from_error)?;

                    let oauth = deserialized_json.0.map_err(
                        |GithubAccessTokenError {
                             error,
                             error_description,
                             error_uri,
                         }| {
                            internal_server_error!(
                                "github endpoint returned error",
                                error,
                                error_description,
                                error_uri,
                            )
                        },
                    )?;

                    if !oauth.scope.is_empty() {
                        return Err(internal_server_error!(
                            "github oauth scopes is not empty",
                            oauth.scope,
                        ));
                    }
                    if !oauth.token_type.eq_ignore_ascii_case("bearer") {
                        return Err(internal_server_error!(
                            "github oauth token type is not bearer",
                            oauth.token_type,
                        ));
                    }

                    Ok(oauth)
                })?;

                let token_created_at = OffsetDateTime::now_utc();

                // TODO: macro or smthn for error_span, in_scope, in_current_span
                let client = error_span!("building octocrab client").in_scope(|| {
                    octocrab::OctocrabBuilder::new()
                        .oauth(octocrab::auth::OAuth {
                            access_token: oauth.access_token.clone().into(),
                            token_type: "bearer".into(),
                            scope: vec![],
                        })
                        .build()
                        .map_err(InternalServerError::from_error)
                })?;

                let user = InternalServerError::wrap(
                    client.current().user(),
                    error_span!("getting current user"),
                )
                .await?;

                let new_session = database
                    .login_user_by_github(
                        user_session.map(|s| s.id),
                        entity::github_auth::Model {
                            // Postgres does not have u64 :(
                            user_id: user.id.0.to_string(),
                            access_token: oauth.access_token,
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
        Ok(Either::E2(Redirect::to(&format!(
            "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
            github.client_id, github.redirect_uri
        ))))
    }
}
