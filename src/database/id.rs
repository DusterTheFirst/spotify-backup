use rspotify::prelude::Id;
use sea_orm::prelude::Uuid;

#[derive(Debug, Clone)]
pub struct SpotifyUserId(String);

impl SpotifyUserId {
    pub fn from_spotify_auth(auth: entity::spotify_auth::Model) -> Self {
        Self(auth.user_id)
    }

    // TODO: do away with
    pub fn from_raw(id: String) -> Self {
        SpotifyUserId(id)
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

    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserSessionId(Uuid);

impl UserSessionId {
    pub fn from_user_session(session: entity::user_session::Model) -> Self {
        Self(session.id)
    }

    pub fn into_uuid(self) -> Uuid {
        self.0
    }

    pub const fn from_raw(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
