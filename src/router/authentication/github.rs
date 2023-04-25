use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::either::Either;
use color_eyre::{
    eyre::{eyre, Context},
    Help, SectionExt,
};
use monostate::MustBe;
use reqwest::header;
use serde::Deserialize;
use time::{Duration, OffsetDateTime};
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
    expires_in: i64, // 28800
    refresh_token: String,
    refresh_token_expires_in: i64, // 15811200
    scope: MustBe!(""),
    token_type: MustBe!("bearer"),
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

                let token = serde_json::from_str::<
                    UntaggedResult<GithubAccessToken, GithubAccessTokenError>,
                >(&token_json)
                .wrap_err("deserializing github access_token response")
                .with_section(|| token_json.header("JSON"))?
                .0
                .map_err(|error| eyre!("github api error: {error:?}"))?;

                let token_created_at = OffsetDateTime::now_utc();
                dbg!(&token);

                let client = octocrab::OctocrabBuilder::new()
                    .oauth(octocrab::auth::OAuth {
                        access_token: token.access_token.clone().into(),
                        token_type: "bearer".to_string(),
                        scope: Vec::new(),
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
                            access_token: token.access_token,
                            expires_at: token_created_at + Duration::seconds(token.expires_in),
                            refresh_token: token.refresh_token,
                            refresh_token_expires_at: token_created_at
                                + Duration::seconds(token.refresh_token_expires_in),
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
