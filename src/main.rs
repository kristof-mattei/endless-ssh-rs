mod cli;
mod client;
mod config;
mod handlers;
mod line;

mod listener;
mod log;
mod statistics;
mod time;

use crate::time::epochms;
use cli::parse_cli;
use client::Client;
use config::Config;
use handlers::set_up_handlers;
use listener::Listener;
use log::logmsg;
use statistics::Statistics;
use std::os::unix::io::AsRawFd;

use libc::ECONNABORTED;
use libc::EINTR;
use libc::EMFILE;
use libc::ENFILE;
use libc::ENOBUFS;
use libc::ENOMEM;
use libc::EPROTO;
use libc::EXIT_FAILURE;

use log::LogLevel;
use std::collections::VecDeque;

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
                c.send_next = now + u128::from(config.delay.get());
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

    // let server = Server::create(config.port.into(), config.bind_family);
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

            statistics.connects += 1;

            match accept {
                Ok((socket, addr)) => {
                    let send_next = epochms() + u128::from(config.delay.get());
                    match socket.set_nonblocking(true) {
                        Ok(_) => {},
                        Err(e) => {
                            eprintln!(
                                "Failed to set client to non-blockign mode, discarding, {}",
                                e
                            );
                            // TODO Close socket (?)
                        },
                    }

                    let client = Client::new(socket, addr, send_next);

                    clients.push_back(client);

                    let client = clients.back().unwrap();

                    let message = format!(
                        "ACCEPT host={} port={} fd={} n={}/{}",
                        client.ipaddr,
                        client.port,
                        client.fd.as_raw_fd(),
                        clients.len(),
                        config.max_clients
                    );

                    logmsg(LogLevel::Info, message);
                },
                Err(e) => {
                    match e.raw_os_error() {
                        Some(EMFILE | ENFILE) => {
                            // config.max_clients = clients.len();
                            // logmsg(LogLevel::Info, format!("MaxClients {}", clients.len()));
                            logmsg(
                                LogLevel::Info,
                                format!("Unable to accept new connection due to {}", e),
                            );
                        },
                        Some(ECONNABORTED | EINTR | ENOBUFS | ENOMEM | EPROTO) => {
                            eprintln!("endless-ssh-rs: warning: {}", e);
                        },
                        _ => {
                            eprintln!("endless-ssh-rs: fatal: {}", e);
                            std::process::exit(EXIT_FAILURE);
                        },
                    }
                },
            }
        }
    }

    let time_spent = destroy_clients(&mut clients);

    statistics.milliseconds += time_spent;

    statistics.log_totals(&[]);

    //     if (logmsg == logsyslog)
    //         closelog();

    Ok(())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_randline() {}
}
