use rspotify::prelude::Id;
use sea_orm::prelude::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubUserId(u64);

impl GithubUserId {
    pub fn from_octocrab(user_id: octocrab::models::UserId) -> Self {
        Self(user_id.0)
    }

    pub fn from_model(model: entity::github_auth::Model) -> Self {
        Self(model.user_id.parse().expect("user id should be an integer"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpotifyUserId(String);

impl SpotifyUserId {
    pub fn from_model(auth: entity::spotify_auth::Model) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountId(Uuid);

impl AccountId {
    pub fn from_model(account: entity::account::Model) -> Self {
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
    pub fn from_model(session: entity::user_session::Model) -> Self {
        Self(session.id)
    }

    pub fn into_uuid(self) -> Uuid {
        self.0
    }

    // TODO: do away with
    pub const fn from_raw(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
