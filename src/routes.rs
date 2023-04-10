use axum::response::Redirect;

pub mod api;
pub mod error;
pub mod index;

pub async fn favicon() -> Redirect {
    Redirect::to("/static/branding/logo@192.png")
}
