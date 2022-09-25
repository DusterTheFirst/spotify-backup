use axum::response::Redirect;

pub async fn auth() -> Redirect {
    Redirect::to(uri)
}
pub async fn auth_redirect() {}
