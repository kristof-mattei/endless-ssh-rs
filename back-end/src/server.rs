use std::future::IntoFuture;
use std::net::SocketAddr;

use axum::Router;
use color_eyre::eyre::Context;
use tokio_util::sync::CancellationToken;
use tracing::{event, Level};

pub(crate) async fn setup_server(
    bind_to: SocketAddr,
    router: Router,
    token: CancellationToken,
) -> Result<(), color_eyre::Report> {
    event!(Level::INFO, ?bind_to, "Trying to bind");

    let listener = tokio::net::TcpListener::bind(bind_to)
        .await
        .wrap_err("Failed to bind server to port")?;

    event!(Level::INFO, ?bind_to, "Server bound successfully");

    axum::serve(listener, router)
        .with_graceful_shutdown(token.cancelled_owned())
        .into_future()
        .await
        .map_err(Into::into)
}
