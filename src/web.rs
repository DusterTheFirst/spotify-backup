use std::{
    convert::Infallible,
    future::{self, Future},
    net::SocketAddr,
    pin::Pin,
    task::{Context, Poll},
};

use flume::{Receiver, Sender};
use hyper::{service::Service, Body, Request, Response, Server, StatusCode};
use indoc::indoc;
use tracing::trace;

use crate::REDIRECT_URL;

#[derive(Debug, Clone)]
pub struct MakeService {
    auth_code_url_tx: Sender<String>,
    web_server_shutdown_tx: Sender<()>,
}

impl<T> Service<T> for MakeService {
    type Response = HttpService;
    type Error = Infallible;
    type Future = future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, _: T) -> Self::Future {
        future::ready(Ok(HttpService(self.clone())))
    }
}

#[derive(Debug)]
pub struct HttpService(MakeService);

impl Service<Request<Body>> for HttpService {
    type Response = Response<Body>;
    type Error = hyper::http::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + Sync>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    #[tracing::instrument(err, skip(self, request), name = "http_service")]
    fn call(&mut self, request: Request<Body>) -> Self::Future {
        let auth_code_url_tx = self.0.auth_code_url_tx.clone();
        let web_server_shutdown_tx = self.0.web_server_shutdown_tx.clone();

        Box::pin(async move {
            trace!("Received request");

            auth_code_url_tx
                .send_async(format!("{}{}", REDIRECT_URL, request.uri()))
                .await
                .unwrap();
            trace!("Sent auth code");

            web_server_shutdown_tx.send_async(()).await.unwrap();
            trace!("Sent shutdown signal");

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", "text/html")
                .body(Body::from(indoc! {"
                    <html>
                        <head>
                            <script>
                                setTimeout(() => window.close(), 1000);
                            </script>
                        </head>
                        <body>
                            <center>This page will automatically close in 1 second</center>
                        </body>
                    </html>
                "}))
        })
    }
}

#[derive(Debug, Clone)]
pub struct OneOffWebServer {
    service: MakeService,
    auth_code_url_rx: Receiver<String>,
    web_server_shutdown_rx: Receiver<()>,
}

impl OneOffWebServer {
    pub fn new() -> Self {
        let (auth_code_url_tx, auth_code_url_rx) = flume::bounded::<String>(1);
        let (web_server_shutdown_tx, web_server_shutdown_rx) = flume::bounded::<()>(0);

        Self {
            auth_code_url_rx,
            service: MakeService {
                auth_code_url_tx,
                web_server_shutdown_tx,
            },
            web_server_shutdown_rx,
        }
    }

    #[tracing::instrument(err, skip(self))]
    pub async fn wait_for_request(&mut self) -> hyper::Result<String> {
        let address = SocketAddr::from(([127, 0, 0, 1], 8080));

        trace!(%address, "Starting web server");

        Server::bind(&address)
            .serve(self.service.clone())
            .with_graceful_shutdown(async {
                self.web_server_shutdown_rx.recv_async().await.unwrap();

                trace!("Shutting down web server");
            })
            .await?;

        let code = self.auth_code_url_rx.recv_async().await.unwrap();
        trace!(%code, "Received auth code");

        Ok(code)
    }
}
