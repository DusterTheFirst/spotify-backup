use axum::response::Redirect;

#[tracing::instrument]
pub async fn healthy() -> &'static str {
    "OK"
}

#[tracing::instrument]
pub async fn panic() {
    panic!("manual api panic")
}

pub async fn auth() -> Redirect {
    Redirect::to("")
}

pub async fn auth_redirect() {}
