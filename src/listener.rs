use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpListener};
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of_mut;
use std::{
    io::{Error, ErrorKind},
    net::TcpStream,
};

use color_eyre::eyre::{self, eyre, Report, WrapErr};
use libc::{poll, pollfd, POLLIN};
use time::OffsetDateTime;
use tracing::{event, Level};

use crate::{
    client::Client,
    client_queue::ClientQueue,
    config::{BindFamily, Config},
    statistics::Statistics,
    wrap_and_report, SIZE_IN_BYTES,
};
use crate::{ffi_wrapper::set_receive_buffer_size, timeout::Timeout};

pub(crate) struct Listener {
    listener: TcpListener,
    fds: pollfd,
}

impl Listener {
    pub(crate) fn start_listening(config: &Config) -> Result<Self, eyre::Report> {
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

        let listener = TcpListener::bind(sa).unwrap();

        listener
            .set_nonblocking(true)
            .wrap_err("Failed to set listener to non-blocking")?;

        event!(Level::DEBUG, ?listener, "Bound and listening!");

        let fd = listener.as_raw_fd();

        Ok(Self {
            listener,
            fds: pollfd {
                fd,
                events: POLLIN,
                revents: 0,
            },
        })
    }

    pub(crate) fn wait_poll(
        &mut self,
        can_accept_more_clients: bool,
        timeout: &Timeout,
    ) -> Result<bool, eyre::Report> {
        // Wait for next event
        event!(
            Level::DEBUG,
            ?timeout,
            "{}",
            if can_accept_more_clients {
                "Waiting for data on socket or timeout expiration"
            } else {
                "Maximum clients reached, just waiting until timeout expires"
            },
        );

        let r = unsafe {
            poll(
                addr_of_mut!(self.fds),
                can_accept_more_clients.into(),
                timeout.as_c_timeout(),
            )
        };

        match r {
            -1 => {
                let last_error = Error::last_os_error();

                // poll & ppoll's EINTR cannot be avoided by using SA_RESTART
                // see https://stackoverflow.com/a/48553220
                if ErrorKind::Interrupted == last_error.kind() {
                    event!(Level::DEBUG, "Poll interrupted");

                    Ok(false)
                } else {
                    Err(Report::from(last_error).wrap_err(
                        "Something went wrong during polling / waiting for the next call",
                    ))
                }
            },
            0 => {
                // ppoll returning 0 means timeout expiration
                event!(Level::DEBUG, "Ending poll because of timeout expiraton");

                Ok(false)
            },
            1 if self.fds.revents & POLLIN == POLLIN => {
                event!(Level::DEBUG, "Ending poll because of incoming connection");

                Ok(true)
            },
            r => Err(eyre!(
                "poll() returned {}, which is impossible, as we only wait on 1 file descriptor",
                r
            )),
        }
    }

    pub(crate) fn accept(
        &self,
        clients: &mut ClientQueue<TcpStream>,
        statistics: &mut Statistics,
        config: &mut Config,
    ) -> Result<(), color_eyre::Report> {
        let accept = self.listener.accept();

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
                    // continue;
                }
                // Set the smallest possible recieve buffer. This reduces local
                // resource usage and slows down the remote end.
                else if let Err(error) = set_receive_buffer_size(&socket, SIZE_IN_BYTES) {
                    event!(
                        Level::ERROR,
                        ?error,
                        "Failed to set the tcp stream's receive buffer",
                    );

                    // can't do anything anymore
                    // continue;
                } else {
                    let client =
                        Client::new(socket, addr, OffsetDateTime::now_utc() + config.delay);

                    clients.push(client);

                    event!(
                        Level::INFO,
                        addr = ?addr,
                        current_clients = clients.len(),
                        max_clients = config.max_clients,
                        "Accepted new client",
                    );
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
                    // FATAL
                    return Err(wrap_and_report!(
                        Level::ERROR,
                        error,
                        "Unable to accept new connection"
                    ));
                },
            },
        }

        Ok(())
    }
}
