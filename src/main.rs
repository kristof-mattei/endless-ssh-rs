mod build_env;
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

use color_eyre::config::HookBuilder;
use color_eyre::eyre;
use dotenvy::dotenv;
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tracing::{Level, event};
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;
use tracing_subscriber::{EnvFilter, Layer as _};

use crate::build_env::get_build_env;
use crate::cli::parse_cli;
use crate::client::Client;
use crate::client_queue::process_clients;
use crate::config::Config;
use crate::listener::listen_for_new_connections;
use crate::statistics::{Statistics, statistics_sigusr1_handler};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

type StdDuration = std::time::Duration;

const SIZE_IN_BYTES: usize = 1;

fn get_config() -> Result<Arc<Config>, eyre::Report> {
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

    Ok(config)
}

fn print_header() {
    const NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    let build_env = get_build_env();

    event!(
        Level::INFO,
        "{} v{} - built for {} ({})",
        NAME,
        VERSION,
        build_env.get_target(),
        build_env.get_target_cpu().unwrap_or("base cpu variant"),
    );
}

async fn start_tasks(config: Arc<Config>) -> Result<(), eyre::Report> {
    print_header();

    // this channel is used to communicate between
    // tasks and this function, in the case that a task fails, they'll send a message on the shutdown channel
    // after which we'll gracefully terminate other services
    let cancellation_token = CancellationToken::new();
    let client_cancellation_token = CancellationToken::new();
    let statistics_cancellation_token = CancellationToken::new();

    let (statistics_sender, statistics_join_handle) =
        Statistics::new(statistics_cancellation_token.clone());

    // clients channel
    let (client_sender, client_receiver) =
        tokio::sync::mpsc::unbounded_channel::<Client<TcpStream>>();

    // available slots semaphore
    let semaphore = Arc::new(Semaphore::new(config.max_clients.into()));

    let tasks = TaskTracker::new();

    {
        tasks.spawn(listen_for_new_connections(
            Arc::clone(&config),
            cancellation_token.clone(),
            client_sender.clone(),
            Arc::clone(&semaphore),
            statistics_sender.clone(),
        ));
    }

    let process_clients_handler = {
        // listen to new connection channel, convert into client, push to client channel
        tasks.spawn(process_clients(
            client_cancellation_token.clone(),
            config.delay,
            config.max_line_length,
            client_sender.clone(),
            client_receiver,
            statistics_sender.clone(),
        ))
    };

    {
        tasks.spawn(statistics_sigusr1_handler(
            cancellation_token.clone(),
            statistics_sender.clone(),
        ));
    }

    tasks.close();

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
        () = cancellation_token.cancelled() => {
            event!(Level::WARN, "Underlying task stopped, stopping all others tasks");
        },
    }

    // backup, in case we forgot a dropguard somewhere
    cancellation_token.cancel();

    client_cancellation_token.cancel();

    if timeout(StdDuration::from_millis(10000), process_clients_handler)
        .await
        .is_err()
    {
        event!(
            Level::ERROR,
            "Client processor didn't stop within allotted time!"
        );
    }

    {
        // cancel the statistics handler now that the client processor is gone
        statistics_cancellation_token.cancel();
        // wait for abort and do a final abort
        statistics_join_handle.await?.log_totals();
    }

    // wait for the other tasks to shut down gracefully
    if timeout(StdDuration::from_millis(10000), tasks.wait())
        .await
        .is_err()
    {
        event!(Level::ERROR, "Tasks didn't stop within allotted time!");
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

    let config = get_config()?;

    // initialize the runtime
    let rt = tokio::runtime::Runtime::new().unwrap();

    // start service
    let result: Result<(), eyre::Report> = rt.block_on(start_tasks(config));

    result
}
