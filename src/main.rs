#![forbid(unsafe_code)]
#![deny(clippy::unwrap_in_result, clippy::unwrap_used)]

use std::{borrow::Cow, env};

use color_eyre::eyre::Context;
use database::Database;
use tracing_subscriber::{prelude::*, EnvFilter};

mod backup;
mod database;
mod environment;
mod pages;
mod router;

fn main() -> Result<(), color_eyre::Report> {
    color_eyre::install()?;

    tracing_subscriber::Registry::default()
        .with(tracing_error::ErrorLayer::default())
        .with(tracing_subscriber::fmt::layer().pretty())
        .with(EnvFilter::from_default_env())
        .with(sentry::integrations::tracing::layer())
        .init();

    // FIXME: the errors have almost no good context, find way to report color_eyre reports
    let _guard = sentry::init(sentry::ClientOptions {
        dsn: env::var("SENTRY_DSN")
            .ok()
            .map(|dsn| dsn.parse().expect("SENTRY_DSN should be a valid DSN")),
        release: Some(git_version::git_version!(args = ["--always", "--abbrev=40"]).into()),
        sample_rate: 1.0,
        traces_sample_rate: 0.0, // TODO: make not 0, but also not spam
        attach_stacktrace: true,
        send_default_pii: true,
        server_name: env::var("FLY_REGION").map(Cow::from).ok(),
        in_app_include: vec!["spotify_backup"],
        // in_app_exclude: todo!(),
        auto_session_tracking: true,
        session_mode: sentry::SessionMode::Request,
        trim_backtraces: true,
        ..Default::default()
    });

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime builder should succeed")
        .block_on(async {
            let database = Database::connect()
                .await
                .wrap_err("failed to setup to database")?;

            let (backup, router) = tokio::join!(
                tokio::spawn(backup::backup(database.clone())),
                tokio::spawn(router::router(database))
            );

            // FIXME: stupid
            router??;
            backup?;

            Ok(())
        })
}
