use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SpotifyAuth::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SpotifyAuth::UserId)
                            .unique_key()
                            .primary_key()
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(SpotifyAuth::AccessToken).string().not_null())
                    .col(
                        ColumnDef::new(SpotifyAuth::ExpiresAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpotifyAuth::RefreshToken)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SpotifyAuth::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Account::Id)
                            .primary_key()
                            .unique_key()
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Account::Spotify)
                            .unique_key()
                            .string()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .to(SpotifyAuth::Table, SpotifyAuth::UserId)
                            .from(Account::Table, Account::Spotify)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Account::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(GithubAuth::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(GithubAuth::Account)
                            .unique_key()
                            .uuid()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .to(Account::Table, Account::Id)
                            .from(GithubAuth::Table, GithubAuth::Account)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(GithubAuth::UserId)
                            .unique_key()
                            .primary_key()
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(GithubAuth::AccessToken).string().not_null())
                    .col(
                        ColumnDef::new(GithubAuth::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(UserSession::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserSession::Id)
                            .primary_key()
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSession::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserSession::LastSeen)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(ColumnDef::new(UserSession::Account).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .to(Account::Table, Account::Id)
                            .from(UserSession::Table, UserSession::Account)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(SpotifyAuth::Table)
                    .table(GithubAuth::Table)
                    .table(Account::Table)
                    .table(UserSession::Table)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum SpotifyAuth {
    Table,
    UserId,
    AccessToken,
    ExpiresAt,
    RefreshToken,
    CreatedAt,
}

#[derive(Iden)]
enum GithubAuth {
    Table,
    Account,
    UserId,
    AccessToken,
    CreatedAt,
}

#[derive(Iden)]
enum Account {
    Table,
    Id,
    Spotify,
    CreatedAt,
}

#[derive(Iden)]
enum UserSession {
    Table,
    Id,
    Account,
    CreatedAt,
    LastSeen,
}
