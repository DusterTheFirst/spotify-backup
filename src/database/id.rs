use rspotify::prelude::Id;
use sea_orm::prelude::Uuid;

#[derive(Debug, Clone)]
pub struct GithubUserId(u64);

impl GithubUserId {
    pub fn from_octocrab(user_id: octocrab::models::UserId) -> Self {
        Self(user_id.0)
    }

    // TODO: do away with
    pub fn from_raw(id: String) -> Self {
        Self(id.parse().expect("github user id should be an integer"))
    }

    pub fn from_github_auth(model: entity::github_auth::Model) -> Self {
        Self(
            model
                .user_id
                .parse()
                .expect("user id should be a non-integer"),
        )
    }
}

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

    // TODO: do away with
    pub const fn from_raw(uuid: Uuid) -> Self {
        Self(uuid)
    }
}
