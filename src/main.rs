use std::{
    os::unix::prelude::AsRawFd,
    sync::atomic::{AtomicBool, Ordering},
};

use tracing::{event, Level};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::cli::parse_cli;
use crate::clients::Clients;
use crate::handlers::set_up_handlers;
use crate::listener::Listener;
use crate::statistics::Statistics;
use crate::{client::Client, time::duration_since_epoch};

mod cli;
mod client;
mod clients;
mod config;
mod ffi_wrapper;
mod handlers;
mod line;
mod listener;
mod statistics;
mod time;

static RUNNING: AtomicBool = AtomicBool::new(true);
static DUMPSTATS: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), anyhow::Error> {
    {
        let builder = tracing_subscriber::fmt::Subscriber::builder();

        let builder = builder.with_env_filter(EnvFilter::from_default_env());

        let subscriber = builder.finish();

        subscriber.try_init()
    }
    .expect("Unable to install global subscriber");

    let mut statistics: Statistics = Statistics::new();

    let mut config = parse_cli().map_err(|e| {
        // this prints the error in color and exits
        // can't do anything else until
        // https://github.com/clap-rs/clap/issues/2914
        // is merged in
        if let Some(clap_error) = e.downcast_ref::<clap::error::Error>() {
            clap_error.exit();
        }

        e
    })?;

    config.log();

    // Install the signal handlers
    set_up_handlers()?;

    let mut clients = Clients::new();

    let listener = Listener::start_listening(&config)?;

    while RUNNING.load(Ordering::SeqCst) {
        if DUMPSTATS.load(Ordering::SeqCst) {
            // print stats requested (SIGUSR1)
            statistics.log_totals(&(*clients));
            DUMPSTATS.store(false, Ordering::SeqCst);
        }

        // Enqueue clients that are due for another message
        let queue_processing_result = clients.process_queue(&config);

        statistics.bytes_sent += queue_processing_result.bytes_sent;
        statistics.milliseconds += queue_processing_result.time_spent;

        let timeout = queue_processing_result.wait_until.into();

        if clients.len() < config.max_clients.get() && listener.wait_poll(timeout)? {
            let accept = listener.accept();

            event!(Level::DEBUG, ?accept, "Incoming connection");

            statistics.connects += 1;

            match accept {
                Ok((socket, addr)) => {
                    let send_next = duration_since_epoch() + config.delay;
                    match socket.set_nonblocking(true) {
                        Ok(_) => {},
                        Err(e) => {
                            event!(
                                Level::WARN,
                                ?e,
                                "Failed to set incoming to non-blocking mode, discarding"
                            );

                            drop(socket);

                            // can't do anything anymore
                            continue;
                        },
                    }

                    let client = Client::new(socket, addr, send_next);

                    clients.push_back(client);

                    let client = clients.back().unwrap();

                    event!(
                        Level::INFO,
                        "ACCEPT host={} port={} fd={} n={}/{}",
                        client.ipaddr,
                        client.port,
                        client.tcp_stream.as_raw_fd(),
                        clients.len(),
                        config.max_clients
                    );
                },
                Err(e) => match e.raw_os_error() {
                    Some(libc::EMFILE) => {
                        // libc::EMFILE is raised when we've reached our per-process
                        // open handles, so we're setting the limit to the current connected clients
                        config.max_clients = clients.len().try_into()?;
                        event!(Level::WARN, ?e, "Unable to accept new connection");
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
                        event!(Level::INFO, ?e, "Unable to accept new connection");
                    },
                    _ => {
                        let error =
                            anyhow::Error::new(e).context("Unable to accept new connection");

                        event!(Level::ERROR, ?error);

                        return Err(error);
                    },
                },
            }
        }
    }

    let time_spent = clients.destroy_clients();

    statistics.milliseconds += time_spent;

    statistics.log_totals(&[]);

    Ok(())
}
