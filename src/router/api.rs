use axum::{extract::OriginalUri, response::Redirect, Json};
use serde_json::{json, Value};

use crate::router::middleware::RequestMetadata;

#[axum::debug_handler]
pub async fn not_found(
    request_metadata: RequestMetadata,
    OriginalUri(uri): OriginalUri,
) -> Json<Value> {
    let endpoint = uri.path();

    Json(json!({
        "error": {
            "message": "endpoint does not exist",
            "endpoint": endpoint,
            "request_id": request_metadata.request_id,
            "region": request_metadata.region,
            "server": request_metadata.server
        }
    }))
}

#[tracing::instrument]
#[axum::debug_handler]
pub async fn healthy() -> &'static str {
    "OK"
}

#[tracing::instrument]
#[axum::debug_handler]
pub async fn panic() {
    panic!("manual api panic")
}

pub async fn auth() -> Redirect {
    Redirect::to("")
}

pub async fn auth_redirect() {}
