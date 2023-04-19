use axum::http::StatusCode;

pub async fn login() -> (StatusCode, &'static str) {
    (
        StatusCode::NOT_IMPLEMENTED,
        StatusCode::NOT_IMPLEMENTED.canonical_reason().expect(""),
    )
}
