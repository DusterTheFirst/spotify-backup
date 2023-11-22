use std::{convert::Infallible, time::Duration};

use futures::StreamExt;
use tokio::time::Instant;
use tracing::Instrument;

use crate::{database::Database, router::authentication::github::GithubAuthentication};

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

    loop {
        // interval.tick().await;

        tracing::info!("processing backups");

        let mut users = std::pin::pin!(match database.list_users().await {
            Ok(users) => users,
            Err(error) => {
                tracing::error!(?error, "unable to list users");
                continue;
            }
        });

        while let Some(account) = users.next().await {
            let account = match account {
                Ok(user) => user,
                Err(error) => {
                    tracing::error!(?error, "unable to acquire next user");
                    continue;
                }
            };

            let span = tracing::error_span!("backup_user", account = ?account.id);
            async move {
                tracing::debug!(account = ?account);

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

                // TODO:
                // spotify_client.current_user_saved_tracks(market)
                // spotify_client.current_user_saved_albums(market)
            }.instrument(span).await;
        }

        panic!();
    }
}
