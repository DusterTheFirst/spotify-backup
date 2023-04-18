use std::{
    env,
    fmt::{Debug, Display},
};

use color_eyre::{eyre::Context, Report};
use entity::{account, prelude::*, spotify_auth, user_session};
use migration::{Migrator, MigratorTrait, OnConflict};
use sea_orm::{
    prelude::*, ActiveValue, ConnectOptions, IntoActiveModel, Iterable, TransactionError,
    TransactionTrait,
};
use time::OffsetDateTime;
use tracing::info;

use crate::router::session::UserSessionId;

mod id {
    use rspotify::prelude::Id;
    use sea_orm::prelude::Uuid;

    #[derive(Debug, Clone)]
    pub struct SpotifyUserId(String);

    impl SpotifyUserId {
        pub fn from_spotify_auth(auth: entity::spotify_auth::Model) -> Self {
            Self(auth.user_id)
        }

        pub fn from_rspotify_user_id(id: rspotify::model::UserId) -> Self {
            Self(id.id().to_string())
        }

        pub fn as_str(&self) -> &str {
            &self.0
        }

        pub fn into_string(self) -> String {
            self.0
        }
    }

    #[derive(Debug)]
    pub struct AccountId(Uuid);

    impl AccountId {
        pub fn from_account(account: entity::account::Model) -> Self {
            Self(account.id)
        }

        pub fn from_session(session: entity::user_session::Model) -> Self {
            Self(session.account)
        }
    }
}

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
}

async fn get_user_session(
    connection: &impl ConnectionTrait,
    session: UserSessionId,
) -> Result<Option<(user_session::Model, account::Model)>, DbErr> {
    let sessions = UserSession::find_by_id(session.as_uuid())
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
        existing_session: Option<UserSessionId>,
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
                                .update_columns(spotify_auth::Column::iter())
                                .to_owned(),
                        )
                        .exec_with_returning(transaction)
                        .await
                        .wrap_err("updating saved spotify authentication details")?;

                    // Get the current account id, if any, and delete the current session
                    let account_id = match existing_session {
                        Some(session_id) => {
                            match get_user_session(transaction, session_id)
                                .await
                                .wrap_err("getting current session")?
                            {
                                Some((session, account)) => {
                                    // Delete the current session
                                    session
                                        .delete(transaction)
                                        .await
                                        .wrap_err("deleting current session")?;

                                    Some(account.id)
                                }
                                None => None,
                            }
                        }
                        None => None,
                    };

                    // First, try to find an account associated with this spotify user
                    let account = Account::find()
                        .filter(account::Column::Spotify.eq(&spotify_id))
                        .one(transaction)
                        .await
                        .wrap_err("finding user already associated with this spotify account")?;

                    let account = if let Some(account) = account {
                        account
                    } else if let Some(account_id) = account_id {
                        // If no account has this spotify user already, add it to the current account
                        Account::update(account::ActiveModel {
                            id: ActiveValue::Set(account_id),
                            spotify: ActiveValue::Set(Some(spotify_id)),
                            ..Default::default()
                        })
                        .exec(transaction)
                        .await
                        .wrap_err("updating existing account")?
                    } else {
                        // If there is no current account, create one
                        Account::insert(account::ActiveModel {
                            id: ActiveValue::Set(Uuid::new_v4()),
                            spotify: ActiveValue::Set(Some(spotify_id)),
                            created: ActiveValue::Set(OffsetDateTime::now_utc()),
                            ..Default::default()
                        })
                        .exec_with_returning(transaction)
                        .await
                        .wrap_err("creating new account")?
                    };

                    // TODO: session pruning periodically
                    let new_session = UserSession::insert(
                        user_session::Model {
                            created: OffsetDateTime::now_utc(),
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
