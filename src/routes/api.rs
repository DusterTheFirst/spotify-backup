use axum::response::Redirect;

pub async fn auth() -> Redirect {
    Redirect::to("")
}

pub async fn auth_redirect() {}
