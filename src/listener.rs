use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6, TcpListener};
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of_mut;

use color_eyre::eyre::{self, eyre, Report, WrapErr};
use libc::{poll, pollfd, POLLIN};
use tracing::{event, Level};

use crate::config::{BindFamily, Config};
use crate::timeout::Timeout;

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

        event!(Level::DEBUG, message = "Bound and listening!", ?listener);

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
            message = if can_accept_more_clients {
                "Waiting for data on socket or timeout expiration"
            } else {
                "Maximum clients reached, just waiting until timeout expires"
            },
            ?timeout,
        );

        let r = unsafe {
            poll(
                addr_of_mut!(self.fds),
                #[cfg(target_arch = "aarch64")]
                u32::from(can_accept_more_clients),
                #[cfg(not(target_arch = "aarch64"))]
                u64::from(can_accept_more_clients),
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
                event!(
                    Level::DEBUG,
                    message = "Ending poll because of timeout expiraton"
                );

                Ok(false)
            },
            1 if self.fds.revents & POLLIN == POLLIN => {
                event!(
                    Level::DEBUG,
                    message = "Ending poll because of incoming connection"
                );

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
    ) -> Result<(std::net::TcpStream, std::net::SocketAddr), std::io::Error> {
        self.listener.accept()
    }
}
