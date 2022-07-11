mod cli;
mod client;
mod config;
mod handlers;
mod line;
mod listener;
mod statistics;
mod time;

use crate::cli::parse_cli;
use crate::client::Client;
use crate::config::Config;
use crate::handlers::set_up_handlers;
use crate::listener::Listener;
use crate::statistics::Statistics;
use crate::time::epochms;

use tracing::event;
use tracing::Level;

use std::collections::VecDeque;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

fn destroy_clients(clients: &mut VecDeque<Client>) -> u128 {
    let mut time_spent = 0;
    for c in clients.drain(..) {
        time_spent += c.destroy();
    }

    time_spent
}

static RUNNING: AtomicBool = AtomicBool::new(true);
static DUMPSTATS: AtomicBool = AtomicBool::new(false);

fn handle_waiting_clients(clients: &mut VecDeque<Client>, config: &Config) -> (i32, u64) {
    let now = epochms();

    let mut bytes_sent = 0;

    while let Some(c) = clients.front() {
        if c.send_next <= now {
            let c = clients.pop_front().unwrap();
            if let Some((mut c, sent)) = c.sendline(config.max_line_length.get()) {
                if let Some(s) = sent {
                    bytes_sent += s;
                }
                c.send_next = now + u128::from(config.delay_ms.get());
                clients.push_back(c);
            }
        } else {
            return (
                i32::try_from(c.send_next - now).expect("Timeout didn't fit i32"),
                bytes_sent,
            );
        }
    }

    (-1, bytes_sent)
}

fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    let mut statistics: Statistics = Statistics {
        bytes_sent: 0,
        milliseconds: 0,
        connects: 0,
    };

    let mut config = Config::default();

    parse_cli(&mut config)?;

    // Log configuration
    config.log();

    // Install the signal handlers
    set_up_handlers()?;

    let mut clients = VecDeque::<Client>::new();

    let listener = Listener::start_listening(&config)?;

    while RUNNING.load(Ordering::SeqCst) {
        if DUMPSTATS.load(Ordering::SeqCst) {
            // print stats requested (SIGUSR1)
            statistics.log_totals(clients.make_contiguous());
            DUMPSTATS.store(false, Ordering::SeqCst);
        }

        // Enqueue clients that are due for another message
        let (timeout, bytes_sent) = handle_waiting_clients(&mut clients, &config);

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
                        client.fd.as_raw_fd(),
                        clients.len(),
                        config.max_clients
                    );
                },
                Err(e) => {
                    match e.raw_os_error() {
                        Some(libc::EMFILE | libc::ENFILE) => {
                            // config.max_clients = clients.len();
                            event!(Level::INFO, ?e, "Unable to accept new connection");
                        },
                        Some(
                            libc::ECONNABORTED
                            | libc::EINTR
                            | libc::ENOBUFS
                            | libc::ENOMEM
                            | libc::EPROTO,
                        ) => {
                            event!(Level::WARN, ?e, "Unable to accept new connection");
                        },
                        _ => {
                            let wrapped =
                                anyhow::Error::new(e).context("Unable to accept new connection");
                            event!(Level::ERROR, ?wrapped);
                            return Err(wrapped);
                        },
                    }
                },
            }
        }
    }

    let time_spent = destroy_clients(&mut clients);

    statistics.milliseconds += time_spent;

    statistics.log_totals(&[]);

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_randline() {}
}
