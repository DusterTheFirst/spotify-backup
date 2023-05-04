use std::{env, fmt::Debug};

use entity::{account, github_auth, prelude::*, spotify_auth, user_session};
use migration::{Migrator, MigratorTrait, OnConflict};
use rspotify::prelude::Id;
use sea_orm::{
    prelude::*, ActiveValue, ConnectOptions, DatabaseTransaction, IntoActiveModel, QueryTrait,
    TransactionError, TransactionTrait,
};
use time::OffsetDateTime;
use tracing::{error_span, info, Instrument};

use crate::{
    internal_server_error,
    pages::InternalServerError,
    router::authentication::{
        github::GithubAuthentication, spotify::SpotifyAuthentication, IncompleteAccount,
        IncompleteUser,
    },
};

use self::id::UserSessionId;

pub mod id;

#[derive(Debug, Clone)]
pub struct Database {
    connection: DatabaseConnection,
}

impl Database {
    pub async fn connect() -> Result<Database, DbErr> {
        let url = env::var("DATABASE_URL").expect("$DB_ENDPOINT should be set");

        info!(?url, "connecting to database");
        let mut options = ConnectOptions::new(url);
        options
            .sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Trace);

        let connection: DatabaseConnection = sea_orm::Database::connect(options).await?;
        info!("migrating database");
        Migrator::up(&connection, None).await?;

        Ok(Database { connection })
    }
}

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn get_current_user(
        &self,
        session: UserSessionId,
    ) -> Result<Option<IncompleteUser>, InternalServerError> {
        let session = get_user_session(&self.connection, session)
            .instrument(error_span!("finding user session and related account"))
            .await?;

        if let Some((session, account)) = session {
            let spotify = InternalServerError::wrap(
                account.find_related(SpotifyAuth).one(&self.connection),
                error_span!("finding associated spotify authentication"),
            )
            .await?;

            let github = InternalServerError::wrap(
                account.find_related(GithubAuth).one(&self.connection),
                error_span!("finding associated spotify authentication"),
            )
            .await?;

            return Ok(Some(IncompleteUser {
                session,
                account: IncompleteAccount {
                    id: account.id,
                    created_at: account.created_at,

                    github: github.map(GithubAuthentication::from_model),
                    spotify: spotify.map(SpotifyAuthentication::from_model),
                },
            }));
        }

        Ok(None)
    }

    #[tracing::instrument(skip(self))]
    pub async fn logout_current_user(
        &self,
        session: crate::router::session::UserSession,
    ) -> Result<crate::router::session::UserSession, InternalServerError> {
        let transaction = self
            .connection
            .transaction(|transaction| Box::pin(logout_user_session(transaction, session.id)))
            .await;

        match transaction {
            Err(TransactionError::Connection(error)) => Err(InternalServerError::from_error(error)),
            Err(TransactionError::Transaction(error)) => Err(error),
            Ok(()) => Ok(crate::router::session::UserSession::remove()),
        }
    }
}

#[tracing::instrument(skip(connection))]
async fn get_user_session(
    connection: &impl ConnectionTrait,
    session: UserSessionId,
) -> Result<Option<(user_session::Model, account::Model)>, InternalServerError> {
    let session = InternalServerError::wrap_in_current_span(
        UserSession::find_by_id(session.into_uuid())
            .find_also_related(Account)
            .one(connection),
    )
    .await?;

    if let Some((session, account)) = session {
        // Based on the DB schema, this should uphold
        // maybe there is a way to do this in sea-orm
        let account = account.ok_or_else(|| {
            internal_server_error!("account should always exist on a user session")
        })?;

        return Ok(Some((session, account)));
    }

    Ok(None)
}

#[tracing::instrument(skip(transaction))]
async fn logout_user_session(
    transaction: &DatabaseTransaction,
    session: UserSessionId,
) -> Result<(), InternalServerError> {
    let user_session = get_user_session(transaction, session).await?;

    if let Some((session, account)) = user_session {
        InternalServerError::wrap(
            session.delete(transaction),
            error_span!("deleting current session"),
        )
        .await?;

        // Delete an incomplete account if this session points to it and is the last session pointing to it
        let other_sessions = InternalServerError::wrap(
            account.find_related(UserSession).one(transaction),
            error_span!("finding other sessions of account"),
        )
        .await?;

        // TODO: consolidate this with IncompleteUser???
        if (account.github.is_none() || account.spotify.is_none()) && other_sessions.is_none() {
            InternalServerError::wrap(
                account.delete(transaction),
                error_span!("deleting current, incomplete account"),
            )
            .await?;
        }
    }

    Ok(())
}

impl Database {
    #[tracing::instrument(skip(self, spotify_auth), fields(spotify = spotify_auth.user_id.id()))]
    pub async fn login_user_by_spotify(
        &self,
        session: Option<UserSessionId>,
        spotify_auth: SpotifyAuthentication,
    ) -> Result<UserSessionId, InternalServerError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let model = spotify_auth.into_model();
                    let spotify_id = model.user_id.clone();

                    // Update the saved spotify authentication details for this user
                    InternalServerError::wrap(
                        SpotifyAuth::insert(model.into_active_model())
                            .on_conflict(
                                OnConflict::column(spotify_auth::Column::UserId)
                                    // Do not update created_at
                                    .update_columns([
                                        spotify_auth::Column::AccessToken,
                                        spotify_auth::Column::ExpiresAt,
                                        spotify_auth::Column::RefreshToken,
                                    ])
                                    .to_owned(),
                            )
                            .exec_with_returning(transaction),
                        error_span!("updating saved spotify authentication details"),
                    )
                    .await?;

                    let account = get_create_or_update_account(
                        transaction,
                        account::ActiveModel {
                            spotify: ActiveValue::Set(Some(spotify_id)),
                            ..Default::default()
                        },
                        session,
                    )
                    .await?;

                    // TODO: session pruning periodically
                    let new_session = InternalServerError::wrap(
                        UserSession::insert(
                            user_session::Model {
                                created_at: OffsetDateTime::now_utc(),
                                last_seen: OffsetDateTime::now_utc(),
                                id: Uuid::new_v4(),
                                account: account.id,
                            }
                            .into_active_model(),
                        )
                        .exec_with_returning(transaction),
                        error_span!("creating new user session"),
                    )
                    .await?;

                    Ok(UserSessionId::from_user_session(new_session))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => InternalServerError::from_error(error),
                TransactionError::Transaction(error) => error,
            })
    }

    #[tracing::instrument(skip(self, github_auth), fields(github = github_auth.user_id.0))]
    pub async fn login_user_by_github(
        &self,
        session: Option<UserSessionId>,
        github_auth: GithubAuthentication,
    ) -> Result<UserSessionId, InternalServerError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let model = github_auth.into_model();
                    let user_id = model.user_id.clone();

                    // Update the saved spotify authentication details for this user
                    InternalServerError::wrap(
                        GithubAuth::insert(model.into_active_model())
                            .on_conflict(
                                OnConflict::column(github_auth::Column::UserId)
                                    // Do not update created_at
                                    .update_columns([github_auth::Column::AccessToken])
                                    .to_owned(),
                            )
                            .exec_with_returning(transaction),
                        error_span!("updating saved github authentication details"),
                    )
                    .await?;

                    let account = get_create_or_update_account(
                        transaction,
                        account::ActiveModel {
                            github: ActiveValue::Set(Some(user_id)),
                            ..Default::default()
                        },
                        session,
                    )
                    .await?;

                    // TODO: session pruning periodically
                    let new_session = InternalServerError::wrap(
                        UserSession::insert(
                            user_session::Model {
                                created_at: OffsetDateTime::now_utc(),
                                last_seen: OffsetDateTime::now_utc(),
                                id: Uuid::new_v4(),
                                account: account.id,
                            }
                            .into_active_model(),
                        )
                        .exec_with_returning(transaction),
                        error_span!("creating new user session"),
                    )
                    .await?;

                    Ok(UserSessionId::from_user_session(new_session))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => InternalServerError::from_error(error),
                TransactionError::Transaction(error) => error,
            })
    }
}

/// This function will invalidate any session given to it, you must recreate a session after running this
// TODO: reduce database calls?
// FIXME: use type system to ensure account only deleted when wanted to be deleted (ie, session deletion account deletion propagation)
#[tracing::instrument(skip(transaction))]
pub async fn get_create_or_update_account(
    transaction: &DatabaseTransaction,
    model: account::ActiveModel,
    session: Option<UserSessionId>,
) -> Result<account::Model, InternalServerError> {
    let spotify_filter = match &model.spotify {
        ActiveValue::Set(Some(spotify)) | ActiveValue::Unchanged(Some(spotify)) => Some(spotify),
        _ => None,
    };

    let github_filter = match &model.github {
        ActiveValue::Set(Some(github)) | ActiveValue::Unchanged(Some(github)) => Some(github),
        _ => None,
    };

    // First, try to find an account associated with this model by filtering
    // by the provided service account
    let account = InternalServerError::wrap(
        Account::find()
            .apply_if(spotify_filter, |q, value| {
                q.filter(account::Column::Spotify.eq(value))
            })
            .apply_if(github_filter, |q, value| {
                q.filter(account::Column::Github.eq(value))
            })
            .one(transaction),
        error_span!("finding user already associated with this spotify account"),
    )
    .await?;

    // If an account already exists with this
    if let Some(account) = account {
        // Delete old session
        if let Some(session_id) = session {
            let session = InternalServerError::wrap(
                UserSession::find_by_id(session_id.into_uuid()).one(transaction),
                error_span!("getting current session"),
            )
            .await?;

            match session {
                Some(session) if account.id == session.account => {
                    // Invalidate the current session, but keep the account around
                    InternalServerError::wrap(
                        session.delete(transaction),
                        error_span!("deleting current session"),
                    )
                    .await?;
                }
                _ => {
                    // Invalidate the current session removing any incomplete accounts
                    logout_user_session(transaction, session_id)
                        .instrument(error_span!("deleting old user session"))
                        .await?;
                }
            }
        }

        return Ok(account);
    }

    // Associate the spotify user with the existing account
    if let Some(session) = session {
        let session = get_user_session(transaction, session).await?;

        if let Some((session, account)) = session {
            // Invalidate the current session, but keep the account around
            InternalServerError::wrap(
                session.delete(transaction),
                error_span!("deleting current session"),
            )
            .await?;

            // If no account has this spotify user already, add it to the current account
            let account = InternalServerError::wrap(
                Account::update(account::ActiveModel {
                    id: ActiveValue::Set(account.id),
                    ..model
                })
                .exec(transaction),
                error_span!("updating existing account"),
            )
            .await?;

            return Ok(account);
        }
    }

    // If there is no current account, create one
    let account = InternalServerError::wrap(
        Account::insert(account::ActiveModel {
            id: ActiveValue::Set(Uuid::new_v4()),
            created_at: ActiveValue::Set(OffsetDateTime::now_utc()),
            ..model
        })
        .exec_with_returning(transaction),
        error_span!("creating new account"),
    )
    .await?;

    Ok(account)
}
