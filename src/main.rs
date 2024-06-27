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
use ffi_wrapper::set_receive_buffer_size;
use time::OffsetDateTime;
use tracing::{event, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use crate::cli::parse_cli;
use crate::client::Client;
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
            Ok(true) => (),
            Ok(false) => continue,
            Err(error) => {
                event!(Level::WARN, ?error, "Something went wrong while polling");

                continue;
            },
        };

        event!(Level::DEBUG, "Trying to accept incoming connection");

        let accept = listener.accept();

        statistics.connects += 1;

        match accept {
            Ok((socket, addr)) => {
                if let Err(error) = socket.set_nonblocking(true) {
                    event!(
                        Level::WARN,
                        ?error,
                        "Failed to set incoming connect to non-blocking mode, discarding",
                    );

                    // can't do anything anymore
                    continue;
                }

                // Set the smallest possible recieve buffer. This reduces local
                // resource usage and slows down the remote end.
                if let Err(error) = set_receive_buffer_size(&socket, SIZE_IN_BYTES) {
                    event!(
                        Level::ERROR,
                        ?error,
                        "Failed to set the tcp stream's receive buffer",
                    );

                    // can't do anything anymore
                    continue;
                };

                let client =
                    Client::initialize(socket, addr, OffsetDateTime::now_utc() + config.delay);

                clients.push(client);

                event!(
                    Level::INFO,
                    addr = ?addr,
                    current_clients = clients.len(),
                    max_clients = config.max_clients,
                    "Accepted new client",
                );
            },
            Err(error) => match error.raw_os_error() {
                Some(libc::EMFILE) => {
                    // libc::EMFILE is raised when we've reached our per-process
                    // open handles, so we're setting the limit to the current connected clients
                    config.max_clients = clients.len().try_into()?;
                    event!(Level::WARN, ?error, "Unable to accept new connection");
                },
                Some(
                    libc::ENFILE
                    | libc::ECONNABORTED
                    | libc::EINTR
                    | libc::ENOBUFS
                    | libc::ENOMEM
                    | libc::EPROTO,
                ) => {
                    // libc::ENFILE: whole system has too many open handles
                    // libc::ECONNABORTED: connection aborted while accepting
                    // libc::EINTR: signal came in while handling this syscall,
                    // libc::ENOBUFS: no buffer space
                    // libc::ENOMEM: no memory
                    // libc::EPROTO: protocol error
                    // all are non fatal
                    event!(Level::INFO, ?error, "Unable to accept new connection");
                },
                _ => {
                    // FATAL
                    return Err(wrap_and_report!(
                        Level::ERROR,
                        error,
                        "Unable to accept new connection"
                    ));
                },
            },
        }
    }

    let time_spent = clients.destroy_clients();

    statistics.time_spent += time_spent;

    statistics.log_totals::<()>(&[]);

    Ok(())
}
