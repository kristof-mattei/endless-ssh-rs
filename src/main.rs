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

use crate::cli::parse_cli;
use crate::client::Client;
use crate::clients::Clients;
use crate::config::Config;
use crate::handlers::set_up_handlers;
use crate::listener::Listener;
use crate::statistics::Statistics;
use crate::time::epochms;

use tracing::event;
use tracing::Level;

use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

static RUNNING: AtomicBool = AtomicBool::new(true);
static DUMPSTATS: AtomicBool = AtomicBool::new(false);

fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let mut statistics: Statistics = Statistics::new();

    let mut config = Config::new();

    parse_cli(&mut config)?;

    config.log();

    // Install the signal handlers
    set_up_handlers()?;

    let mut clients = Clients::new();

    let listener = Listener::start_listening(&config)?;

    while RUNNING.load(Ordering::SeqCst) {
        if DUMPSTATS.load(Ordering::SeqCst) {
            // print stats requested (SIGUSR1)
            statistics.log_totals(clients.make_contiguous());
            DUMPSTATS.store(false, Ordering::SeqCst);
        }

        // Enqueue clients that are due for another message
        let (timeout, bytes_sent) = clients.process_queue(&config);

        statistics.bytes_sent += bytes_sent;

        if clients.len() < config.max_clients.get() && listener.wait_poll(timeout)? {
            let accept = listener.accept();

            event!(Level::DEBUG, ?accept, "Incoming connection");

            statistics.connects += 1;

            match accept {
                Ok((socket, addr)) => {
                    let send_next = epochms() + u128::from(config.delay_ms.get());
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
                        let wrapped =
                            anyhow::Error::new(e).context("Unable to accept new connection");
                        event!(Level::ERROR, ?wrapped);
                        return Err(wrapped);
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

#[cfg(test)]
mod tests {

    #[test]
    fn test_randline() {}
}
