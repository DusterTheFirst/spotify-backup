use std::{convert::Infallible, time::Duration};

use futures::{StreamExt, TryStreamExt};
use rspotify::clients::OAuthClient;
use tokio::time::Instant;
use tracing::{error_span, Level};

use crate::{
    database::Database,
    pages::InternalServerError,
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
                    Ok(account) => {
                        let _ = backup_user(account).await;
                    }
                    Err(error) => {
                        tracing::error!(?error, "unable to acquire next user");
                    }
                }
            })
            .await;

        panic!();
    }
}

#[tracing::instrument(skip_all, fields(account = %account.id), err(level = Level::WARN))]
async fn backup_user(account: Account) -> Result<(), InternalServerError> {
    let github = match account.github.as_ref().map(GithubAuthentication::as_client) {
        Some(Ok(github)) => github,
        Some(Err(error)) => {
            // tracing::warn!(%error, "failed to create github client from authentication details");

            return Err(error);
        }
        None => {
            tracing::trace!("incomplete user, missing github account... skipping");

            return Ok(());
        }
    };
    let spotify_client = account.spotify.as_client();

    // TODO: do this a bit smarter... somehow
    let mut downloaded_tracks = 0;
    let mut saved_tracks = Vec::new();
    loop {
        let start = Instant::now();
        let mut page = InternalServerError::wrap(
            spotify_client.current_user_saved_tracks_manual(
                None,
                Some(rspotify::DEFAULT_PAGINATION_CHUNKS),
                Some(downloaded_tracks),
            ),
            error_span!("fetch_page"),
        )
        .await?;

        downloaded_tracks += page.items.len() as u32;
        let total_tracks: u32 = page.total;

        let request_nr = downloaded_tracks.div_ceil(rspotify::DEFAULT_PAGINATION_CHUNKS);
        let total_requests = total_tracks.div_ceil(rspotify::DEFAULT_PAGINATION_CHUNKS);

        tracing::trace!(
            request = format_args!("{request_nr}/{total_requests}"),
            song = format_args!("{downloaded_tracks}/{total_tracks}"),
            percent = format_args!(
                "{:.2}%",
                100.0 * downloaded_tracks as f32 / total_tracks as f32
            ),
            elapsed = ?start.elapsed()
        );

        saved_tracks.append(&mut page.items);

        if page.next.is_none() {
            break;
        }
    }

    tokio::fs::write("target/tracks_test.rs", format!("{saved_tracks:#?}")).await;
    // TODO:
    // spotify_client.current_user_saved_albums(market)

    Ok(())
}
