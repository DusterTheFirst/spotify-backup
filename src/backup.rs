use std::{convert::Infallible, time::Duration};

use futures::{StreamExt, TryStreamExt};
use rspotify::clients::OAuthClient;
use tokio::time::Instant;

use crate::{
    database::Database,
    router::authentication::{github::GithubAuthentication, Account},
};

#[tracing::instrument(skip_all)]
pub async fn backup(database: Database) -> Infallible {
    let now = time::OffsetDateTime::now_utc();
    let instant_now = Instant::now();

    let next_hour =
        now.replace_time(time::Time::MIDNIGHT + time::Duration::hours(now.hour() as i64 + 1));
    let time_till_next_hour = (next_hour - now).unsigned_abs();

    tracing::debug!(%next_hour, ?time_till_next_hour, "waiting until next hour");

    let hour = Duration::from_secs(60 * 60);
    let mut interval = tokio::time::interval_at(instant_now + time_till_next_hour, hour);

    #[allow(clippy::never_loop)]
    loop {
        // interval.tick().await;

        tracing::info!("processing backups");

        let concurrency = 20;

        // handle users concurrently (since we have to sequentially fetch the paginated liked songs)
        database
            .list_users(concurrency)
            .await
            .for_each_concurrent(Some(concurrency as usize), |account| async {
                match account {
                    Ok(account) => backup_user(account).await,
                    Err(error) => {
                        tracing::error!(?error, "unable to acquire next user");
                    }
                }
            })
            .await;

        panic!();
    }
}

#[tracing::instrument(skip_all, fields(account = %account.id))]
async fn backup_user(account: Account) {
    let github = match account.github.as_ref().map(GithubAuthentication::as_client) {
        Some(Ok(github)) => github,
        Some(Err(error)) => {
            tracing::warn!(%error, "failed to create github client from authentication details");

            return;
        }
        None => {
            tracing::trace!("incomplete user, missing github account... skipping");

            return;
        }
    };
    let spotify_client = account.spotify.as_client();

    // TODO: do this a bit smarter... somehow
    // TODO: show progress ... somehow
    let saved_tracks = spotify_client
        .current_user_saved_tracks(None)
        .try_collect::<Vec<_>>()
        .await;

    tokio::fs::write("target/tracks_test.rs", format!("{saved_tracks:#?}")).await;
    // TODO:
    // spotify_client.current_user_saved_albums(market)
}
