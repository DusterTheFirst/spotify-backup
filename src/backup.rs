use std::{convert::Infallible, time::Duration};

use futures::StreamExt;
use tokio::time::Instant;

use crate::database::Database;

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
        interval.tick().await;

        tracing::info!("processing backups");

        let mut users = match database.list_users().await {
            Ok(users) => users,
            Err(error) => {
                tracing::error!(?error, "unable to list users");
                continue;
            }
        };

        while let Some(user) = users.next().await {
            let user = match user {
                Ok(user) => user,
                Err(error) => {
                    tracing::error!(?error, "unable to acquire next user");
                    continue;
                }
            };
        }
    }
}
