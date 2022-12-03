use axum::{
    body::{Body, BoxBody},
    extract::{Host, State},
    http::{
        uri::{Authority, Parts, Scheme},
        Request, Response, Uri,
    },
    middleware::Next,
    response::{IntoResponse, Redirect},
};
use tracing::{trace, trace_span};

pub mod catch_panic;
pub mod trace;

pub async fn redirect_to_domain(
    State(expected_host): State<Authority>,
    Host(host): Host,
    req: Request<Body>,
    next: Next<Body>,
) -> Response<BoxBody> {
    if host == expected_host.as_ref() {
        next.run(req).await
    } else {
        trace_span!("redirect_to_domain", %expected_host, %host).in_scope(|| {
            let mut parts = Parts::default();
            // Inherit path and query from request
            parts.path_and_query = req.uri().path_and_query().cloned();

            parts.authority = Some(expected_host);
            parts.scheme = Some(if cfg!(debug_assertions) {
                Scheme::HTTP
            } else {
                Scheme::HTTPS
            });

            trace!("URI authority did not match configured HOST");

            Redirect::permanent(
                &Uri::from_parts(parts)
                    .expect("redirect uri should be a valid uri")
                    .to_string(),
            )
            .into_response()
        })
    }
}
