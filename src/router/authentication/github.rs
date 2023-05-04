use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use octocrab::models::UserId;
use reqwest::header;
use secrecy::{ExposeSecret, Secret, SecretString};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, error_span, trace, Instrument};

use crate::{
    environment::GITHUB_ENVIRONMENT,
    internal_server_error,
    pages::InternalServerError,
    router::{session::UserSession, AppState},
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
#[serde(untagged, deny_unknown_fields)]
enum GithubAccessTokenResponse {
    Success {
        access_token: String,
        scope: String,
        token_type: String,
    },
    Failure {
        error: String,
        error_description: String,
        error_uri: String,
    },
}

pub async fn login(
    State(AppState { database, reqwest }): State<AppState>,
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
                                ("client_id", GITHUB_ENVIRONMENT.client_id.clone()),
                                ("client_secret", GITHUB_ENVIRONMENT.client_secret.clone()),
                                ("redirect_uri", GITHUB_ENVIRONMENT.redirect_uri.to_string()),
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

                let token_json = InternalServerError::wrap(
                    response.text(),
                    error_span!("receiving github access_token response"),
                )
                .await?;

                let access_token = error_span!(
                    "deserializing github access_token response",
                    json = token_json
                )
                .in_scope(|| {
                    let deserialized_json: GithubAccessTokenResponse =
                        serde_json::from_str(&token_json)
                            .map_err(InternalServerError::from_error)?;

                    match deserialized_json {
                        GithubAccessTokenResponse::Success {
                            access_token,
                            scope,
                            token_type,
                        } => {
                            if !scope.is_empty() {
                                return Err(internal_server_error!(
                                    "github oauth scopes is not empty",
                                    scope,
                                ));
                            }
                            if !token_type.eq_ignore_ascii_case("bearer") {
                                return Err(internal_server_error!(
                                    "github oauth token type is not bearer",
                                    token_type,
                                ));
                            }

                            Ok(Secret::new(access_token))
                        }
                        GithubAccessTokenResponse::Failure {
                            error,
                            error_description,
                            error_uri,
                        } => Err(internal_server_error!(
                            "github endpoint returned error",
                            error,
                            error_description,
                            error_uri,
                        )),
                    }
                })?;

                let auth = GithubAuthentication::create(access_token).await?;

                let span = error_span!("logging in github account", user.id = auth.user_id.0);
                let new_session = database
                    .login_user_by_github(user_session.map(|s| s.id), auth)
                    .instrument(span)
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
            GITHUB_ENVIRONMENT.client_id, GITHUB_ENVIRONMENT.redirect_uri
        ))))
    }
}

#[derive(Debug)]
pub struct GithubAuthentication {
    access_token: SecretString,

    pub user_id: UserId,
    pub created_at: OffsetDateTime,
}

impl GithubAuthentication {
    async fn create(access_token: Secret<String>) -> Result<Self, InternalServerError> {
        let mut auth = Self {
            access_token,
            user_id: UserId(u64::MAX), // Populate later
            created_at: OffsetDateTime::now_utc(),
        };

        let current_user = InternalServerError::wrap(
            auth.as_client()?.current().user(),
            error_span!("fetching current user"),
        )
        .await?;

        auth.user_id = current_user.id; // Populates here

        Ok(auth)
    }

    #[tracing::instrument(skip(self))]
    pub fn as_client(&self) -> Result<octocrab::Octocrab, InternalServerError> {
        octocrab::OctocrabBuilder::new()
            .oauth(octocrab::auth::OAuth {
                access_token: self.access_token.clone(),
                token_type: "bearer".into(),
                scope: vec![],
            })
            .build()
            .map_err(InternalServerError::from_error)
    }

    pub fn into_model(self) -> entity::github_auth::Model {
        entity::github_auth::Model {
            user_id: self.user_id.to_string(),
            access_token: self.access_token.expose_secret().clone(),
            created_at: self.created_at,
        }
    }

    pub fn from_model(model: entity::github_auth::Model) -> Self {
        Self {
            access_token: model.access_token.into(),
            user_id: UserId(
                model
                    .user_id
                    .parse()
                    .expect("user id should never be a non-integer"),
            ),
            created_at: model.created_at,
        }
    }
}
