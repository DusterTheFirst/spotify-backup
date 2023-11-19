use axum::{
    extract::{Query, State},
    response::Redirect,
};
use octocrab::models::UserId;
use reqwest::header;
use secrecy::{ExposeSecret, Secret, SecretString};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{debug, error_span, trace, warn, Instrument};

use crate::{
    database::{id::AccountId, GithubAccountAlreadyTakenError},
    environment::GITHUB_ENVIRONMENT,
    internal_server_error,
    pages::InternalServerError,
    router::AppState,
};

use super::User;

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

pub async fn logout(
    State(AppState { database, .. }): State<AppState>,
    user: Option<User>,
) -> Result<Redirect, InternalServerError> {
    let user = match user {
        Some(user) => user,
        None => {
            warn!("user attempted to logout of github while logged out");
            return Ok(Redirect::to("/account"));
        }
    };

    database.remove_github_from_account(user).await?;

    Ok(Redirect::to("/account"))
}

pub async fn login(
    State(AppState { database, reqwest }): State<AppState>,
    query: Option<Query<GithubAuthCodeResponse>>,
    user: Option<User>,
) -> Result<Redirect, InternalServerError> {
    let user = match user {
        Some(user) => user,
        None => {
            warn!("user attempted to add github while logged out");
            return Ok(Redirect::to("/account"));
        }
    };

    let auth_code_response = match query {
        Some(Query(auth)) => auth,
        None => {
            return Ok(Redirect::to(&format!(
                "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}",
                GITHUB_ENVIRONMENT.client_id, GITHUB_ENVIRONMENT.redirect_uri
            )));
        }
    };

    match auth_code_response {
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
                    serde_json::from_str(&token_json).map_err(InternalServerError::from_error)?;

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

            let span = error_span!("logging in github account", github.id = auth.user_id.0);
            match database
                .associate_github_to_account(user, auth)
                .instrument(span)
                .await?
            {
                Ok(()) => Ok(Redirect::to("/account")),
                Err(GithubAccountAlreadyTakenError) => todo!("account already taken"),
            }
        }
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

    pub fn into_model(self, account: AccountId) -> entity::github_auth::Model {
        entity::github_auth::Model {
            account: account.into_uuid(),
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
