use std::{
    env,
    fmt::{Debug, Display},
};

use color_eyre::{eyre::Context, Report};
use entity::{account, github_auth, prelude::*, spotify_auth, user_session};
use migration::{Migrator, MigratorTrait, OnConflict};
use sea_orm::{
    prelude::*, ActiveValue, ConnectOptions, DatabaseTransaction, IntoActiveModel, QueryTrait,
    TransactionError, TransactionTrait,
};
use time::OffsetDateTime;
use tracing::info;

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
            .sqlx_logging_level(tracing::log::LevelFilter::Info);

        let connection: DatabaseConnection = sea_orm::Database::connect(options).await?;
        info!("migrating database");
        Migrator::up(&connection, None).await?;

        Ok(Database { connection })
    }
}

impl Database {
    pub async fn get_user_session(
        &self,
        session: UserSessionId,
    ) -> Result<Option<(user_session::Model, account::Model)>, DbErr> {
        get_user_session(&self.connection, session).await
    }

    pub async fn logout_user_session(
        &self,
        session: crate::router::session::UserSession,
    ) -> Result<crate::router::session::UserSession, Report> {
        let transaction = self
            .connection
            .transaction::<_, _, DbErrReport>(|transaction| {
                Box::pin(logout_user_session(transaction, session.id))
            })
            .await;

        match transaction {
            Err(TransactionError::Connection(error)) => Err(Report::new(error)),
            Err(TransactionError::Transaction(error)) => Err(error.0),
            Ok(()) => Ok(crate::router::session::UserSession::remove()),
        }
    }
}

async fn get_user_session(
    connection: &impl ConnectionTrait,
    session: UserSessionId,
) -> Result<Option<(user_session::Model, account::Model)>, DbErr> {
    let sessions = UserSession::find_by_id(session.into_uuid())
        .find_also_related(Account)
        .one(connection)
        .await?;

    Ok(sessions.map(|(session, account)| {
        (
            session,
            // Based on the DB schema, this should uphold
            // maybe there is a way to do this in sea-orm
            account.expect("account should always exist on a user session"),
        )
    }))
}

async fn logout_user_session(
    transaction: &DatabaseTransaction,
    session: UserSessionId,
) -> Result<(), DbErrReport> {
    let user_session = get_user_session(transaction, session)
        .await
        .wrap_err("getting current session")?;

    if let Some((session, account)) = user_session {
        session
            .delete(transaction)
            .await
            .wrap_err("deleting current session")?;

        // Delete an incomplete account if this session points to it and is the last session pointing to it
        let no_other_sessions = account
            .find_related(UserSession)
            .one(transaction)
            .await
            .wrap_err("finding other sessions of account")?
            .is_none();

        // TODO: consolidate this with IncompleteUser???
        if (account.github.is_none() || account.spotify.is_none()) && no_other_sessions {
            account
                .delete(transaction)
                .await
                .wrap_err("deleting current, incomplete account")?;
        }
    }

    Ok(())
}

pub struct DbErrReport(Report);

impl std::error::Error for DbErrReport {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Display for DbErrReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Debug for DbErrReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl From<Report> for DbErrReport {
    fn from(value: Report) -> Self {
        Self(value)
    }
}

impl Database {
    pub async fn login_user_by_spotify(
        &self,
        session: Option<UserSessionId>,
        spotify_auth: entity::spotify_auth::Model, // TODO: do not expose?
    ) -> Result<UserSessionId, Report> {
        self.connection
            .transaction::<_, _, DbErrReport>(|transaction| {
                Box::pin(async move {
                    let spotify_id = spotify_auth.user_id.clone();

                    // Update the saved spotify authentication details for this user
                    SpotifyAuth::insert(spotify_auth.into_active_model())
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
                        .exec_with_returning(transaction)
                        .await
                        .wrap_err("updating saved spotify authentication details")?;

                    let account = get_create_or_update_account(
                        transaction,
                        account::ActiveModel {
                            spotify: ActiveValue::Set(Some(spotify_id)),
                            ..Default::default()
                        },
                        session,
                    )
                    .await
                    .wrap_err("updating account")?;

                    // TODO: session pruning periodically
                    let new_session = UserSession::insert(
                        user_session::Model {
                            created_at: OffsetDateTime::now_utc(),
                            last_seen: OffsetDateTime::now_utc(),
                            id: Uuid::new_v4(),
                            account: account.id,
                        }
                        .into_active_model(),
                    )
                    .exec_with_returning(transaction)
                    .await
                    .wrap_err("creating new user session")?;

                    Ok(UserSessionId::from_user_session(new_session))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => Report::new(error),
                TransactionError::Transaction(error) => error.0,
            })
    }

    pub async fn login_user_by_github(
        &self,
        session: Option<UserSessionId>,
        github_auth: entity::github_auth::Model, // TODO: do not expose?
    ) -> Result<UserSessionId, Report> {
        self.connection
            .transaction::<_, _, DbErrReport>(|transaction| {
                Box::pin(async move {
                    let user_id = github_auth.user_id.clone();

                    // Update the saved spotify authentication details for this user
                    GithubAuth::insert(github_auth.into_active_model())
                        .on_conflict(
                            OnConflict::column(github_auth::Column::UserId)
                                // Do not update created_at
                                .update_columns([github_auth::Column::AccessToken])
                                .to_owned(),
                        )
                        .exec_with_returning(transaction)
                        .await
                        .wrap_err("updating saved github authentication details")?;

                    let account = get_create_or_update_account(
                        transaction,
                        account::ActiveModel {
                            github: ActiveValue::Set(Some(user_id)),
                            ..Default::default()
                        },
                        session,
                    )
                    .await
                    .wrap_err("updating account")?;

                    // TODO: session pruning periodically
                    let new_session = UserSession::insert(
                        user_session::Model {
                            created_at: OffsetDateTime::now_utc(),
                            last_seen: OffsetDateTime::now_utc(),
                            id: Uuid::new_v4(),
                            account: account.id,
                        }
                        .into_active_model(),
                    )
                    .exec_with_returning(transaction)
                    .await
                    .wrap_err("creating new user session")?;

                    Ok(UserSessionId::from_user_session(new_session))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => Report::new(error),
                TransactionError::Transaction(error) => error.0,
            })
    }
}

/// This function will invalidate any session given to it, you must recreate a session after running this
// TODO: reduce database calls?
// FIXME: use type system to ensure account only deleted when wanted to be deleted (ie, session deletion account deletion propagation)
pub async fn get_create_or_update_account(
    transaction: &DatabaseTransaction,
    model: account::ActiveModel,
    session: Option<UserSessionId>,
) -> Result<account::Model, DbErrReport> {
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
    let account = Account::find()
        .apply_if(spotify_filter, |q, value| {
            q.filter(account::Column::Spotify.eq(value))
        })
        .apply_if(github_filter, |q, value| {
            q.filter(account::Column::Github.eq(value))
        })
        .one(transaction)
        .await
        .wrap_err("finding user already associated with this spotify account")?;

    // If an account already exists with this
    if let Some(account) = account {
        // Delete old session
        if let Some(session_id) = session {
            let session = UserSession::find_by_id(session_id.into_uuid())
                .one(transaction)
                .await
                .wrap_err("getting current session")?;

            match session {
                Some(session) if account.id == session.account => {
                    // Invalidate the current session, but keep the account around
                    session
                        .delete(transaction)
                        .await
                        .wrap_err("deleting current session")?;
                }
                _ => {
                    // Invalidate the current session removing any incomplete accounts
                    logout_user_session(transaction, session_id)
                        .await
                        .wrap_err("deleting old user session")?;
                }
            }
        }

        return Ok(account);
    }

    // Associate the spotify user with the existing account
    if let Some(session) = session {
        let session = get_user_session(transaction, session)
            .await
            .wrap_err("getting current session")?;

        if let Some((session, account)) = session {
            // Invalidate the current session, but keep the account around
            session
                .delete(transaction)
                .await
                .wrap_err("deleting current session")?;

            // If no account has this spotify user already, add it to the current account
            let account = Account::update(account::ActiveModel {
                id: ActiveValue::Set(account.id),
                ..model
            })
            .exec(transaction)
            .await
            .wrap_err("updating existing account")?;

            return Ok(account);
        }
    }

    // If there is no current account, create one
    let account = Account::insert(account::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        created_at: ActiveValue::Set(OffsetDateTime::now_utc()),
        ..model
    })
    .exec_with_returning(transaction)
    .await
    .wrap_err("creating new account")?;

    Ok(account)
}
