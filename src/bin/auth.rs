use lambda_http::Request;
use lambda_runtime::{Context, Error};
use log::{info, trace, LevelFilter};
use rspotify::{scopes, AuthCodeSpotify, Credentials};
use serde_json::Value;
use simplelog::{ColorChoice, TermLogger, TerminalMode};

#[tokio::main]
async fn main() -> Result<(), Error> {
    TermLogger::init(
        LevelFilter::Trace,
        simplelog::Config::default(),
        TerminalMode::Mixed,
        ColorChoice::Always,
    )?;

    lambda_runtime::run(lambda_http::handler(handler)).await?;
    Ok(())
}

async fn handler(request: Request, _ctx: Context) -> Result<Value, Error> {
    // Load the spotify credentials
    let creds = Credentials::from_env().expect("no rspotify credentials");

    info!("Setup spotify credentials");
    trace!("Redirect url: {}", _ctx.env_config.endpoint);

    let mut spotify = AuthCodeSpotify::with_config(
        creds,
        rspotify::OAuth {
            scopes: scopes!("user-library-read"),
            redirect_uri: _ctx.env_config.endpoint.into(),
            ..Default::default()
        },
        rspotify::Config {
            token_cached: true,
            token_refreshing: true,
            ..Default::default()
        },
    );

    Ok(serde_json::to_value(request.into_body())?)
}
