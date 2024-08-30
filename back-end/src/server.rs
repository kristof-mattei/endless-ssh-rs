use std::future::IntoFuture;
use std::net::SocketAddr;

use axum::Router;
use tokio_util::sync::CancellationToken;
use tracing::{event, Level};

pub(crate) async fn server_forever(bind_to: SocketAddr, router: Router, token: CancellationToken) {
    event!(Level::INFO, ?bind_to, "Trying to bind");

    let listener = match tokio::net::TcpListener::bind(bind_to).await {
        Ok(listener) => listener,
        Err(err) => {
            event!(Level::ERROR, ?err, "Failed to bind server to port");
            return;
        },
    };

    event!(Level::INFO, ?bind_to, "Server bound successfully");

    let server = axum::serve(listener, router)
        .with_graceful_shutdown(token.cancelled_owned())
        .into_future();

    match server.await {
        Ok(()) => {
            event!(Level::INFO, "Server shut down gracefully");
        },
        Err(err) => {
            event!(Level::ERROR, ?err, "Server died");
        },
    }
}
