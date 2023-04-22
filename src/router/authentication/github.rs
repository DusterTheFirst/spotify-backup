use axum::{http::StatusCode, extract::State};

use crate::GithubEnvironment;

pub async fn login(State(state): State<GithubEnvironment>) -> (StatusCode, &'static str) {
    (
        StatusCode::NOT_IMPLEMENTED,
        StatusCode::NOT_IMPLEMENTED.canonical_reason().expect(""),
    )
}
