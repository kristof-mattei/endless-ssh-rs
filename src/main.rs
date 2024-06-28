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

use std::env;
use std::sync::atomic::{AtomicBool, Ordering};

use dotenvy::dotenv;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::cli::parse_cli;
use crate::client_queue::ClientQueue;
use crate::listener::Listener;
use crate::statistics::Statistics;

static RUNNING: AtomicBool = AtomicBool::new(true);
static DUMPSTATS: AtomicBool = AtomicBool::new(false);

const SIZE_IN_BYTES: usize = 1;

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), color_eyre::Report> {
    // set up .env
    let _r = dotenv();

    color_eyre::config::HookBuilder::default()
        .capture_span_trace_by_default(false)
        .install()?;

    let rust_log_value = env::var(EnvFilter::DEFAULT_ENV)
        .unwrap_or_else(|_| format!("DEBUG,{}=TRACE", env!("CARGO_PKG_NAME").replace('-', "_")));

    // set up logger
    tracing_subscriber::registry()
        .with(EnvFilter::builder().parse_lossy(rust_log_value))
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_error::ErrorLayer::default())
        .init();

    let mut statistics: Statistics = Statistics::new();

    let mut config = parse_cli().map_err(|error| {
        // this prints the error in color and exits
        // can't do anything else until
        // https://github.com/clap-rs/clap/issues/2914
        // is merged in
        if let Some(clap_error) = error.downcast_ref::<clap::error::Error>() {
            clap_error.exit();
        }

        error
    })?;

    config.log();

    // Install the signal handlers
    signal_handlers::setup_handlers()?;

    let mut clients = ClientQueue::new();

    let mut listener = Listener::start_listening(&config)?;

    while RUNNING.load(Ordering::SeqCst) {
        if DUMPSTATS.load(Ordering::SeqCst) {
            // print stats requested (SIGUSR1)
            statistics.log_totals(&(*clients));
            DUMPSTATS.store(false, Ordering::SeqCst);
        }

        // Enqueue clients that are due for another message
        let queue_processing_result = clients.process_queue(&config);

        statistics.bytes_sent += queue_processing_result.bytes_sent;
        statistics.time_spent += queue_processing_result.time_spent;

        let timeout = queue_processing_result.timeout.into();

        match listener.wait_poll(clients.len() < config.max_clients.get(), &timeout) {
            Ok(true) => {
                event!(Level::DEBUG, "Trying to accept incoming connection");

                listener.accept(&mut clients, &mut statistics, &mut config)?;
            },
            Ok(false) => continue,
            Err(error) => {
                event!(Level::WARN, ?error, "Something went wrong while polling");

                continue;
            },
        };
    }

    let time_spent = clients.destroy_clients();

    statistics.time_spent += time_spent;

    statistics.log_totals::<()>(&[]);

    Ok(())
}
