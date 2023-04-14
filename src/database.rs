use std::{env, fmt::Debug};

use entity::{account, prelude::*, spotify_auth, user_session};
use migration::{Migrator, MigratorTrait, OnConflict};
use rspotify::prelude::Id;
use sea_orm::{prelude::*, ActiveValue, ConnectOptions, IntoActiveModel, Iterable};
use time::OffsetDateTime;
use tracing::info;

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

impl SpotifyId {
    pub fn from_raw(id: rspotify::model::UserId) -> Self {
        Self(id.id().to_string())
    }
}

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

#[derive(Debug)]
pub struct AccountId(Uuid);

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn get_or_create_account_by_spotify(
        &self,
        spotify: SpotifyId,
    ) -> Result<account::Model, DbErr> {
        dbg!(
            Account::insert(account::ActiveModel {
                spotify: ActiveValue::Set(Some(spotify.0)),
                ..Default::default()
            })
            .on_conflict(
                OnConflict::column(account::Column::Spotify)
                    .update_column(account::Column::Spotify)
                    .to_owned(),
            )
            .exec_with_returning(&self.connection)
            .await
        )
    }
}
