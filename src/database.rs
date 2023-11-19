use std::{env, fmt::Debug};

use entity::{account, github_auth, prelude::*, spotify_auth, user_session};
use futures::{Stream, TryStreamExt};
use migration::{Migrator, MigratorTrait, OnConflict};
use rspotify::prelude::Id;
use sea_orm::{
    prelude::*, ConnectOptions, DatabaseTransaction, DeleteResult, IntoActiveModel,
    TransactionError, TransactionTrait,
};
use time::OffsetDateTime;
use tracing::{error_span, info, Instrument};

use crate::{
    internal_server_error,
    pages::InternalServerError,
    router::authentication::{
        self, github::GithubAuthentication, spotify::SpotifyAuthentication, User,
    },
};

use self::id::{AccountId, UserSessionId};

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

pub struct LazyUser<'db> {
    connection: &'db DatabaseConnection,

    model: account::Model,
}

impl<'db> LazyUser<'db> {
    pub fn account(&self) -> &account::Model {
        &self.model
    }

    #[tracing::instrument(skip_all, fields(account = ?self.model.id))]
    pub async fn spotify(&self) -> Result<spotify_auth::Model, InternalServerError> {
        InternalServerError::wrap(
            self.model.find_related(SpotifyAuth).one(self.connection),
            error_span!("retrieving spotify auth details", account_id = ?self.model.id, spotify_user = self.model.spotify),
        )
        .await?
        .ok_or_else(|| internal_server_error!("assuming a complete user"))
    }

    #[tracing::instrument(skip_all, fields(account = ?self.model.id))]
    pub async fn github(&self) -> Result<github_auth::Model, InternalServerError> {
        InternalServerError::wrap(
            self.model.find_related(GithubAuth).one(self.connection),
            error_span!("retrieving github auth details", account_id = ?self.model.id),
        )
        .await?
        .ok_or_else(|| internal_server_error!("assuming a complete user")) // FIXME: bad assumption
    }
}

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn list_users(
        &self,
    ) -> Result<impl Stream<Item = Result<LazyUser, DbErr>> + Send + '_, InternalServerError> {
        // let b = Account::find()
        //     .join(
        //         sea_orm::JoinType::LeftJoin,
        //         account::Relation::SpotifyAuth.def(),
        //     )
        //     .column_as(spotify_auth::Column::AccessToken, "spotify_access_token")
        //     .join(
        //         sea_orm::JoinType::LeftJoin,
        //         account::Relation::GithubAuth.def(),
        //     )
        //     .column_as(github_auth::Column::AccessToken, "github_access_token")
        //     .build(sea_orm::DatabaseBackend::Postgres)
        //     .to_string();

        // tracing::info!("{b}");

        Ok(InternalServerError::wrap(
            Account::find().stream(&self.connection),
            error_span!("finding all accounts"),
        )
        .await?
        .map_ok(|model| LazyUser {
            connection: &self.connection,
            model,
        }))
    }
}

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn get_current_user(
        &self,
        session: UserSessionId,
    ) -> Result<Option<User>, InternalServerError> {
        let session = get_user_session(&self.connection, session)
            .instrument(error_span!("finding user session and related account"))
            .await?;

        if let Some((session, account)) = session {
            let spotify = InternalServerError::wrap(
                account.find_related(SpotifyAuth).one(&self.connection),
                error_span!("finding associated spotify authentication"),
            )
            .await?
            .ok_or_else(|| {
                internal_server_error!("spotify authentication must exist for a user")
            })?;

            let github = InternalServerError::wrap(
                account.find_related(GithubAuth).one(&self.connection),
                error_span!("finding associated spotify authentication"),
            )
            .await?;

            return Ok(Some(User {
                session,
                account: authentication::Account {
                    created_at: account.created_at,
                    id: AccountId::from_model(account),

                    spotify: SpotifyAuthentication::from_model(spotify),

                    github: github.map(GithubAuthentication::from_model),
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

    #[tracing::instrument(skip_all, fields(account = ?user.account.id))]
    pub async fn delete_current_user(
        &self,
        user: User,
    ) -> Result<crate::router::session::UserSession, InternalServerError> {
        let transaction = self
            .connection
            .transaction(|transaction| {
                Box::pin(async move {
                    // Deleting the spotify authentication will cascade to the user account
                    let DeleteResult { rows_affected } = InternalServerError::wrap(
                        SpotifyAuth::delete(user.account.spotify.into_model().into_active_model())
                            .exec(transaction),
                        error_span!("deleting user"),
                    )
                    .await?;

                    if rows_affected == 0 {
                        tracing::warn!("attempted to delete a user that did not exist");
                    }

                    Ok(())
                })
            })
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
    let DeleteResult { rows_affected } = InternalServerError::wrap(
        UserSession::delete_by_id(session.into_uuid()).exec(transaction),
        error_span!("deleting current session"),
    )
    .await?;

    if rows_affected == 0 {
        tracing::warn!(?session, "attempted to delete a session that did not exist");
    }

    Ok(())
}

impl Database {
    #[tracing::instrument(skip(self, spotify_auth), fields(spotify = spotify_auth.user_id.id()))]
    pub async fn login_user(
        &self,
        spotify_auth: SpotifyAuthentication,
    ) -> Result<UserSessionId, InternalServerError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let model = spotify_auth.into_model();
                    let spotify_id = model.user_id.clone();

                    // Update the saved spotify authentication details for this user
                    let spotify_auth = InternalServerError::wrap(
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
                        error_span!(
                            "updating saved spotify authentication details",
                            spotify = spotify_id
                        ),
                    )
                    .await?;

                    let account = InternalServerError::wrap(
                        spotify_auth.find_related(Account).one(transaction),
                        error_span!("fetching account for spotify user", spotify_id),
                    )
                    .await?;

                    let account = match account {
                        Some(existing) => existing,
                        None => {
                            InternalServerError::wrap(
                                Account::insert(
                                    account::Model {
                                        id: Uuid::new_v4(),
                                        spotify: spotify_id.clone(),
                                        created_at: OffsetDateTime::now_utc(),
                                    }
                                    .into_active_model(),
                                )
                                .exec_with_returning(transaction),
                                error_span!("creating new account", spotify_id),
                            )
                            .await?
                        }
                    };

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
                        error_span!("creating new user session", account = ?account.id),
                    )
                    .await?;

                    Ok(UserSessionId::from_model(new_session))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => InternalServerError::from_error(error),
                TransactionError::Transaction(error) => error,
            })
    }

    #[tracing::instrument(skip_all, fields(github = github_auth.user_id.0, user = ?user.account.id))]
    pub async fn associate_github_to_account(
        &self,
        user: User,
        github_auth: GithubAuthentication,
    ) -> Result<Result<(), GithubAccountAlreadyTakenError>, InternalServerError> {
        self.connection
            .transaction(|transaction| {
                Box::pin(async move {
                    let github_auth = github_auth.into_model(user.account.id);
                    let github_id = github_auth.user_id.clone();

                    // Update the saved github authentication details for this user
                    let github_auth = InternalServerError::wrap(
                        GithubAuth::insert(github_auth.into_active_model())
                            .on_conflict(
                                OnConflict::column(github_auth::Column::UserId)
                                    // Do not update created_at
                                    .update_columns([github_auth::Column::AccessToken])
                                    .to_owned(),
                            )
                            .exec_with_returning(transaction),
                        error_span!("updating saved github authentication details", ?github_id),
                    )
                    .await?;

                    if github_auth.account != user.account.id.into_uuid() {
                        return Ok(Err(GithubAccountAlreadyTakenError));
                    }

                    Ok(Ok(()))
                })
            })
            .await
            .map_err(|error| match error {
                TransactionError::Connection(error) => InternalServerError::from_error(error),
                TransactionError::Transaction(error) => error,
            })
    }

    #[tracing::instrument(skip_all, fields(user = ?user.account.id))]
    pub async fn remove_github_from_account(&self, user: User) -> Result<(), InternalServerError> {
        if let Some(github) = user.account.github {
            InternalServerError::wrap_in_current_span(
                GithubAuth::delete_by_id(github.user_id.to_string()).exec(&self.connection),
            )
            .await?;
        }

        Ok(())
    }
}

pub struct GithubAccountAlreadyTakenError;
