use std::{collections::HashSet, convert::Infallible, env, fmt::Debug};

use migration::Migrator;
use sea_orm::{ConnectOptions, DatabaseConnection};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::info;

use crate::router::session::{Account, UserSession};

#[derive(Debug, Clone)]
pub struct Database {
    connection: DatabaseConnection,
}

impl Database {
    #[tracing::instrument]
    pub async fn connect() -> Database {
        let url = env::var("DATABASE_URL").expect("$DB_ENDPOINT should be set");

        let mut options = ConnectOptions::new(url);
        options
            .sqlx_logging(true)
            .sqlx_logging_level(tracing::log::LevelFilter::Info);

        info!(?url, "connecting to database");
        let connection: DatabaseConnection = sea_orm::Database::connect(options);
        Migrator::up(&connection, None).await?;

        Database { connection }
    }
}

impl Database {
    #[tracing::instrument(skip(self))]
    pub async fn healthy(&self) -> Result<(), sea_orm::ExecResult> {
        self.connection.health().await
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SpotifyToken {
    pub access_token: String,
    #[serde(with = "time::serde::timestamp")]
    pub expires_at: OffsetDateTime,
    pub refresh_token: String,
    pub scopes: HashSet<String>,
}

impl Database {
    #[tracing::instrument(skip(self))]
    // TODO: should never need, get this from an account instead
    pub async fn get_user_authentication(
        &self,
        id: SpotifyId,
    ) -> Result<Option<SpotifyToken>, surrealdb::Error> {
        self.connection.select::<Option<_>>(id).await
    }

    #[tracing::instrument(skip(self))]
    pub async fn set_user_authentication(
        &self,
        id: SpotifyId,
        token: SpotifyToken,
    ) -> Result<WithId<SpotifyToken, SpotifyId>, surrealdb::Error> {
        self.connection.update::<Option<_>>(id).content(token).await
    }
}

macro_rules! into_resource {
    ($id:ident => $resource:ident for $type:ident) => {
        impl IntoResource<Option<$resource>> for $type {
            fn into_resource(self) -> surrealdb::Result<Resource> {
                Ok(self.into())
            }
        }

        impl IntoResource<Option<Option<$resource>>> for $type {
            fn into_resource(self) -> surrealdb::Result<Resource> {
                Ok(self.into())
            }
        }

        impl IntoResource<Option<WithId<$resource, $id>>> for $type {
            fn into_resource(self) -> surrealdb::Result<Resource> {
                Ok(self.into())
            }
        }

        impl IntoResource<Option<Option<WithId<$resource, $id>>>> for $type {
            fn into_resource(self) -> surrealdb::Result<Resource> {
                Ok(self.into())
            }
        }
    };
}

macro_rules! named_thing {
    ($(
        $table_name:literal $table:ident:$id:ident => $resource:ident,
    )+) => {
        $(
            #[derive(Debug, Clone, Serialize, Deserialize)]
            #[serde(try_from = "Thing", into = "Thing")]
            pub struct $id(Id);

            impl $id {
                pub const TABLE: $table = $table {};

                pub fn to_raw(&self) -> String {
                    self.0.to_raw()
                }

                pub fn from_raw(id: impl Into<Id>) -> Self {
                    Self(id.into())
                }
            }

            impl From<$id> for Thing {
                fn from(value: $id) -> Self {
                    Thing {
                        tb: String::from($table_name),
                        id: value.0
                    }
                }
            }

            impl TryFrom<Thing> for $id {
                type Error = String;

                fn try_from(value: Thing) -> Result<Self, Self::Error> {
                    if value.tb == $table_name {
                        Ok(Self(value.id))
                    } else {
                        Err(format!("table '{}' does not match expected table '{}'", value.tb, $table_name))
                    }
                }
            }

            impl From<$id> for Resource {
                fn from(value: $id) -> Self {
                    Resource::RecordId(value.into())
                }
            }

            into_resource!($id => $resource for $id);

            #[derive(Debug, Clone, Copy)]
            #[non_exhaustive]
            pub struct $table { }

            impl From<$table> for Resource {
                fn from(_value: $table) -> Self {
                    Resource::Table(Table(String::from($table_name)))
                }
            }

            into_resource!($id => $resource for $table);
        )+
    };
}

named_thing! [
    "sessions" UserSessionTable:UserSessionId => UserSession,
    "accounts" AccountTable:AccountId => PartialAccount,
    "spotify_authentication" SpotifyTable:SpotifyId => SpotifyToken,
    "github_authentication" GithubTable:GithubId => Infallible,
];

#[derive(Debug, Serialize, Deserialize)]
pub struct WithId<D, ID> {
    pub id: ID,
    #[serde(flatten)]
    pub data: D,
}

impl Database {
    // TODO: session pruning periodically
    #[tracing::instrument(skip(self))]
    pub async fn create_user_session(
        &self,
    ) -> Result<WithId<UserSession, UserSessionId>, surrealdb::Error> {
        self.connection
            .create::<Option<_>>(UserSessionId::TABLE)
            .content(UserSession::new())
            .await
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_user_session(
        &self,
        session: UserSessionId,
    ) -> Result<Option<WithId<UserSession, UserSessionId>>, surrealdb::Error> {
        self.connection.select::<Option<_>>(session).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde()]
pub struct PartialAccount {
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub spotify: Option<SpotifyId>,
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub github: Option<GithubId>,
}

impl PartialAccount {
    pub fn into_account(self) -> Option<Account> {
        match (self.spotify, self.github) {
            (Some(spotify), Some(github)) => Some(Account { spotify, github }),
            (_, _) => None,
        }
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
