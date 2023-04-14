use std::{collections::HashSet, env, fmt::Debug};

use entity::{account, spotify_auth, user_session};
use migration::{Migrator, MigratorTrait, OnConflict};
use nutype::nutype;
use rspotify::prelude::Id;
use sea_orm::{prelude::*, ActiveValue, ConnectOptions, IntoActiveModel, Iterable};
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, PrimitiveDateTime};
use tracing::info;

use crate::router::session::Account;

pub use entity::prelude::*;

#[derive(Debug, Clone)]
pub struct Database {
    connection: DatabaseConnection,
}

impl Database {
    #[tracing::instrument]
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

#[derive(Debug)]
pub struct SpotifyId(String);

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn update_user_authentication(
        &self,
        id: SpotifyId,
        auth: spotify_auth::Model, // TODO: do not expose?
    ) -> Result<spotify_auth::Model, DbErr> {
        SpotifyAuth::insert(auth.into_active_model())
            .on_conflict(
                OnConflict::new()
                    .update_columns(spotify_auth::Column::iter())
                    .to_owned(),
            )
            .exec_with_returning(&self.connection)
            .await
    }
}

#[derive(Debug)]
pub struct UserSessionId(Uuid);

impl Database {
    // TODO: session pruning periodically
    #[tracing::instrument(skip(self))]
    pub async fn create_user_session(&self) -> Result<user_session::Model, DbErr> {
        UserSession::insert(user_session::ActiveModel {
            created: ActiveValue::Set(OffsetDateTime::now_utc()),
            last_seen: ActiveValue::Set(OffsetDateTime::now_utc()),
            // TODO: generate UUID?
            ..Default::default()
        })
        .exec_with_returning(&self.connection)
        .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_user_session(
        &self,
        session: UserSessionId,
    ) -> Result<Option<(user_session::Model, Option<account::Model>)>, DbErr> {
        let mut sessions = UserSession::find_by_id(session.0)
            .find_also_related(Account)
            .all(&self.connection)
            .await?;

        // There should only be one
        Ok(sessions.pop())
    }
}

impl Database {
    pub async fn get_or_create_account_by_spotify(
        &self,
        spotify: SpotifyId,
    ) -> Result<WithId<PartialAccount, AccountId>, surrealdb::Error> {
        let mut result = self
            .connection
            .query("BEGIN TRANSACTION")
            // TODO: in some setup I guess
            .query("DEFINE INDEX account_spotify_unique ON TABLE account COLUMNS spotify UNIQUE")
            // TODO: first try to fetch existing account by spotify
            .query("SELECT id FROM account WHERE spotify = $spotify")
            .query("CREATE account SET spotify = $spotify")
            .query("COMMIT TRANSACTION")
            .bind(("spotify", spotify))
            .await?;

        dbg!(result);

        // result.take(index)

        Ok(todo!())
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_account_spotify_id(
        &self,
        account: AccountId,
        spotify: SpotifyId,
    ) -> Result<(), surrealdb::Error> {
        self.connection
            .update::<Option<PartialAccount>>(account)
            .merge(PartialAccount {
                spotify: Some(spotify),
                github: None,
            })
            .await?;

        Ok(())
    }
}
