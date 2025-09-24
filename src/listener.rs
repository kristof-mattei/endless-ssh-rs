use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::sync::Arc;

use color_eyre::eyre;
use time::OffsetDateTime;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::Sender;
use tokio::sync::{RwLock, Semaphore, TryAcquireError};
use tokio_util::sync::CancellationToken;
use tracing::{Level, event};

use crate::SIZE_IN_BYTES;
use crate::client::Client;
use crate::config::{BindFamily, Config};
use crate::ffi_wrapper::set_receive_buffer_size;
use crate::statistics::Statistics;

struct Listener<'c> {
    config: &'c Config,
    listener: TcpListener,
}

pub async fn listen_forever(
    config: Arc<Config>,
    token: CancellationToken,
    client_sender: tokio::sync::mpsc::Sender<Client<TcpStream>>,
    semaphore: Arc<Semaphore>,
    statistics: Arc<RwLock<Statistics>>,
) {
    let _guard = token.clone().drop_guard();

    // listen forever, accept new clients
    let listener = match Listener::bind(&config).await {
        Ok(l) => l,
        Err(error) => {
            event!(Level::ERROR, ?error);
            return;
        },
    };

    event!(Level::INFO, message = "Bound and listening!", listener=?listener.listener);

    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => {
                break;
            },
            result = listener.accept(&client_sender, &semaphore, &statistics) => {
                if let Err(error) = result {
                    event!(Level::ERROR, ?error);

                    // TODO properly log errors
                    break;
                }
            },
        }
    }
}

impl<'c> Listener<'c> {
    pub async fn bind(config: &'c Config) -> Result<Self, eyre::Report> {
        let sa = match config.bind_family {
            BindFamily::Ipv4 => {
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, config.port.get()))
            },
            BindFamily::Ipv6 | BindFamily::DualStack => SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::UNSPECIFIED,
                config.port.get(),
                0,
                0,
            )),
        };

        // TODO BindFamily::Ipv6 is not respected. Dual stack / IPv6 only are
        // set by /proc/sys/net/ipv6/bindv6only

        let listener = TcpListener::bind(sa).await?;

        Ok(Self { config, listener })
    }

    pub async fn accept(
        &self,
        client_sender: &Sender<Client<TcpStream>>,
        semaphore: &Semaphore,
        statistics: &RwLock<Statistics>,
    ) -> Result<(), eyre::Report> {
        let accept = self.listener.accept().await;

        {
            let mut guard = statistics.write().await;
            guard.connects += 1;
        }

        match accept {
            Ok((socket, addr)) => {
                // Set the smallest possible recieve buffer. This reduces local
                // resource usage and slows down the remote end.
                if let Err(error) = set_receive_buffer_size(&socket, SIZE_IN_BYTES) {
                    event!(
                        Level::ERROR,
                        ?error,
                        "Failed to set the tcp stream's receive buffer",
                    );
                } else {
                    // we do try_acquire because either we can add the client or we cannot
                    // no in-between, no sense in waiting
                    match semaphore.try_acquire() {
                        Ok(permit) => {
                            let client = Client::new(
                                socket,
                                addr,
                                OffsetDateTime::now_utc() + self.config.delay,
                            );

                            // we have a permit, we can send it on the queue
                            client_sender.send(client).await?;

                            permit.forget();

                            let current_clients =
                                self.config.max_clients.get() - semaphore.available_permits();

                            event!(
                                Level::INFO,
                                addr = ?addr,
                                current_clients,
                                max_clients = self.config.max_clients,
                                "Accepted new client",
                            );
                        },
                        Err(TryAcquireError::NoPermits) => {
                            event!(Level::WARN, ?addr, "Queue full, not accepting new client");
                        },
                        Err(error @ TryAcquireError::Closed) => {
                            return Err(eyre::Report::new(error)
                                .wrap_err("Queue gone, not accepting new client"));
                        },
                    }
                }
            },
            Err(error) => match error.raw_os_error() {
                Some(libc::EMFILE) => {
                    // libc::EMFILE is raised when we've reached our per-process
                    // open handles, so we're setting the limit to the current connected clients
                    // config.max_clients = clients.len().try_into()?;
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
                    return Err(
                        eyre::Report::new(error).wrap_err("Unable to accept new connection")
                    );
                },
            },
        }

        Ok(())
    }
}
