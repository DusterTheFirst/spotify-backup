use axum::{
    body::{Body, BoxBody},
    extract::{Host, State},
    http::{
        uri::{Authority},
        Request,
    },
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
            trace!("URI authority did not match configured DOMAIN");

            Redirect::permanent(&format!("//{}{}", domain.as_str(), req.uri().path()))
                .into_response()
        })
    }
}
