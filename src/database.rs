use std::env;

use surrealdb::{
    engine::remote::ws::{self, Ws},
    opt::auth::Root,
    Surreal,
};
use tracing::info;

pub type Database = Surreal<ws::Client>;

#[tracing::instrument]
pub async fn connect() -> Database {
    let endpoint = env::var("SURREAL_ENDPOINT").expect("$SURREAL_ENDPOINT should be set");

    info!(?endpoint, "connecting to database");
    let db = Surreal::new::<Ws>(endpoint)
        .await
        .expect("database connection should be established");

    info!("signing in to database");
    db.signin(Root {
        username: &env::var("SURREAL_USER").expect("$SURREAL_USER should be set"),
        password: &env::var("SURREAL_PASS").expect("$SURREAL_PASS should be set"),
    })
    .await
    .expect("provided database credentials should be valid");

    db.use_ns("spotify-backup")
        .use_db("spotify-backup")
        .await
        .expect("namespace and database selection should succeed");

    db
}
