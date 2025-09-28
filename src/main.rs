mod cli;
mod client;
mod client_queue;
mod config;
mod ffi_wrapper;
mod helpers;
mod line;
mod listener;
mod sender;
mod signal_handlers;
mod statistics;
mod timeout;
mod traits;
mod utils;

use std::env::{self, VarError};
use std::sync::Arc;
use std::time::Duration;

use color_eyre::config::HookBuilder;
use color_eyre::eyre;
use dotenvy::dotenv;
use tokio::net::TcpStream;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{Level, event};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};

use crate::cli::parse_cli;
use crate::client::Client;
use crate::client_queue::process_clients_forever;
use crate::listener::listen_forever;
use crate::statistics::Statistics;

const SIZE_IN_BYTES: usize = 1;

async fn start_tasks() -> Result<(), eyre::Report> {
    let name = env!("CARGO_PKG_NAME");
    let version = env!("CARGO_PKG_VERSION");

    event!(
        Level::INFO,
        "{} v{} - built for {}-{}",
        name,
        version,
        std::env::var("TARGETARCH")
            .as_deref()
            .unwrap_or("unknown-arch"),
        std::env::var("TARGETVARIANT")
            .as_deref()
            .unwrap_or("base variant")
    );

    let statistics = Arc::new(RwLock::new(Statistics::new()));

    let config = Arc::new(parse_cli().inspect_err(|error| {
        // this prints the error in color and exits
        // can't do anything else until
        // https://github.com/clap-rs/clap/issues/2914
        // is merged in
        if let Some(clap_error) = error.downcast_ref::<clap::error::Error>() {
            clap_error.exit();
        }
    })?);

    config.log();

    // clients channel
    let (client_sender, client_receiver) =
        tokio::sync::mpsc::channel::<Client<TcpStream>>(config.max_clients.into());

    // available slots semaphore
    let semaphore = Arc::new(Semaphore::new(config.max_clients.into()));

    // this channel is used to communicate between
    // tasks and this function, in the case that a task fails, they'll send a message on the shutdown channel
    // after which we'll gracefully terminate other services
    let token = CancellationToken::new();

    let tasks = TaskTracker::new();

    {
        tasks.spawn(listen_forever(
            Arc::clone(&config),
            token.clone(),
            client_sender.clone(),
            Arc::clone(&semaphore),
            Arc::clone(&statistics),
        ));
    }

    {
        // listen to new connection channel, convert into client, push to client channel
        tasks.spawn(process_clients_forever(
            Arc::clone(&config),
            token.clone(),
            client_sender.clone(),
            client_receiver,
            Arc::clone(&semaphore),
            Arc::clone(&statistics),
        ));
    }

    {
        let token = token.clone();
        let statistics = Arc::clone(&statistics);

        tasks.spawn(async move {
            let _guard = token.clone().drop_guard();

            while let Ok(()) = signal_handlers::wait_for_sigusr1().await {
                statistics.read().await.log_totals::<(), _>(&[]);
            }
        });
    }

    // now we wait forever for either
    // * SIGTERM
    // * ctrl + c (SIGINT)
    // * a message on the shutdown channel, sent either by the server task or
    // another task when they complete (which means they failed)
    tokio::select! {
        result = signal_handlers::wait_for_sigterm() => {
            if let Err(error) = result {
                event!(Level::ERROR, ?error, "Failed to register SIGERM handler, aborting");
            } else {
                // we completed because ...
                event!(Level::WARN, "Sigterm detected, stopping all tasks");
            }
        },
        result = signal_handlers::wait_for_sigint() => {
            if let Err(error) = result {
                event!(Level::ERROR, ?error, "Failed to register CTRL+C handler, aborting");
            } else {
                // we completed because ...
                event!(Level::WARN, "CTRL+C detected, stopping all tasks");
            }
        },
        () = token.cancelled() => {
            event!(Level::WARN, "Underlying task stopped, stopping all others tasks");
        },
    }

    // backup, in case we forgot a dropguard somewhere
    token.cancel();

    tasks.close();

    // wait for the task that holds the server to exit gracefully
    // it listens to shutdown_send
    if timeout(Duration::from_millis(10000), tasks.wait())
        .await
        .is_err()
    {
        event!(Level::ERROR, "Tasks didn't stop within allotted time!");
    }

    {
        (statistics.read().await).log_totals::<(), _>(&[]);
    }

    event!(Level::INFO, "Goodbye");

    Ok(())
}

fn build_default_filter() -> EnvFilter {
    EnvFilter::builder()
        .parse(format!(
            "DEBUG,{}=TRACE,tower_http::trace=TRACE",
            env!("CARGO_CRATE_NAME")
        ))
        .expect("Default filter should always work")
}

fn init_tracing() -> Result<(), eyre::Report> {
    let (filter, filter_parsing_error) = match env::var(EnvFilter::DEFAULT_ENV) {
        Ok(user_directive) => match EnvFilter::builder().parse(user_directive) {
            Ok(filter) => (filter, None),
            Err(error) => (build_default_filter(), Some(eyre::Report::new(error))),
        },
        Err(VarError::NotPresent) => (build_default_filter(), None),
        Err(error @ VarError::NotUnicode(_)) => {
            (build_default_filter(), Some(eyre::Report::new(error)))
        },
    };

    let registry = tracing_subscriber::registry();

    #[cfg(feature = "tokio-console")]
    let registry = registry.with(console_subscriber::ConsoleLayer::builder().spawn());

    registry
        .with(tracing_subscriber::fmt::layer().with_filter(filter))
        .with(tracing_error::ErrorLayer::default())
        .try_init()?;

    filter_parsing_error.map_or(Ok(()), Err)
}

fn main() -> Result<(), eyre::Report> {
    // set up .env, if it fails, user didn't provide any
    let _r = dotenv();

    HookBuilder::default()
        .capture_span_trace_by_default(true)
        .display_env_section(false)
        .install()?;

    init_tracing()?;

    // initialize the runtime
    let rt = tokio::runtime::Runtime::new().unwrap();

    // start service
    let result: Result<(), eyre::Report> = rt.block_on(start_tasks());

    result
}
