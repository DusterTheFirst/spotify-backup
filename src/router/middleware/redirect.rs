use axum::{
    body::{Body, BoxBody},
    extract::{Host, State},
    http::{uri::Authority, Request, Uri},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tracing::{trace, trace_span};

pub async fn redirect_to_domain(
    State(domain): State<Authority>,
    Host(hostname): Host,
    req: Request<Body>,
    next: Next<Body>,
) -> Response<BoxBody> {
    if hostname == domain {
        next.run(req).await
    } else {
        trace_span!("redirect_to_domain", ?domain, %hostname).in_scope(|| {
            // Inherit path and query from request
            let mut parts = req.uri().clone().into_parts();
            parts.authority = Some(domain);

            trace!("URI authority did not match configured DOMAIN");

            let uri = Uri::from_parts(parts).expect("redirect URI should be a valid URI");

            Redirect::permanent(&uri.to_string()).into_response()
        })
    }
}
