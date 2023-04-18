//! `SeaORM` Entity. Generated by sea-orm-codegen 0.11.2

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "account")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub spotify: Option<String>,
    #[sea_orm(unique)]
    pub github: Option<String>,
    pub created: TimeDateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::github_auth::Entity",
        from = "Column::Github",
        to = "super::github_auth::Column::UserId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    GithubAuth,
    #[sea_orm(
        belongs_to = "super::spotify_auth::Entity",
        from = "Column::Spotify",
        to = "super::spotify_auth::Column::UserId",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    SpotifyAuth,
    #[sea_orm(has_many = "super::user_session::Entity")]
    UserSession,
}

impl Related<super::github_auth::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::GithubAuth.def()
    }
}

impl Related<super::spotify_auth::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SpotifyAuth.def()
    }
}

impl Related<super::user_session::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserSession.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
