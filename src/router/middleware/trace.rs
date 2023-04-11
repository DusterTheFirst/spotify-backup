use axum::http::Request;
use tower_http::{request_id::RequestId, trace::MakeSpan};
use tracing::{debug_span, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanMaker;

impl<B> MakeSpan<B> for SpanMaker {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method();
        let uri = request.uri();
        let version = request.version();
        let id = request
            .extensions()
            .get::<RequestId>()
            .expect("request should contain request ID")
            .header_value()
            .to_str()
            .expect("request id should be valid utf-8");

        debug_span!("request", %method, %uri, ?version, %id)
    }
}
