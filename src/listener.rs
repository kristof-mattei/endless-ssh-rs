use std::fmt::Display;
use std::io::Error;
use std::io::ErrorKind;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::net::SocketAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use std::net::TcpListener;
use std::ops::Deref;
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of_mut;

use anyhow::Context;
use anyhow::Result;
use libc::poll;
use libc::pollfd;
use libc::POLLIN;
use time::Duration;
use tracing::event;
use tracing::Level;

use crate::config::BindFamily;
use crate::config::Config;
use crate::wrap_and_report;

pub(crate) enum Timeout {
    Infinite,
    Duration(Duration),
}

impl std::fmt::Debug for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infinite => write!(f, "Infinite"),
            Self::Duration(arg0) => write!(f, "{}", arg0),
        }
    }
}

impl Timeout {
    pub(crate) fn as_c_timeout(&self) -> i32 {
        // note the + 1
        // Duration stores data as seconds and nanoseconds internally.
        // if the nanoseconds < 1 milliseconds it gets lost
        // so we add one to make sure we always wait until the duration has passed
        match self {
            Timeout::Infinite => -1,
            Timeout::Duration(m) => i32::try_from(m.whole_milliseconds() + 1).unwrap_or(i32::MAX),
        }
    }
}

impl Display for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.as_c_timeout()))
    }
}

impl From<Option<Duration>> for Timeout {
    fn from(duration: Option<Duration>) -> Self {
        match duration {
            None => Timeout::Infinite,
            Some(d) => Timeout::Duration(d),
        }
    }
}

#[derive(Debug)]
pub(crate) struct Listener(TcpListener);

impl Deref for Listener {
    type Target = TcpListener;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Listener {
    pub(crate) fn start_listening(config: &Config) -> Result<Self, anyhow::Error> {
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
            .context("Failed to set listener to non-blocking")?;

        event!(Level::DEBUG, message = "Bound and listening!", ?listener);

        Ok(Self(listener))
    }

    pub(crate) fn wait_poll(
        &self,
        can_accept_more_clients: bool,
        timeout: &Timeout,
    ) -> Result<bool, anyhow::Error> {
        // Wait for next event
        let mut fds: pollfd = pollfd {
            fd: self.0.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

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
                addr_of_mut!(fds),
                u64::from(can_accept_more_clients),
                timeout.as_c_timeout(),
            )
        };

        if r == -1 {
            let last_error = Error::last_os_error();
            // // poll & ppoll's EINTR cannot be avoided by using SA_RESTART
            // // see https://stackoverflow.com/a/48553220
            if ErrorKind::Interrupted == last_error.kind() {
                event!(Level::DEBUG, "Poll interrupted, but that's ok");
                return Ok(false);
            }

            return Err(wrap_and_report!(
                Level::ERROR,
                last_error,
                "Something went wrong during polling / waiting for the next call"
            ));
        }

        if fds.revents & POLLIN == POLLIN {
            event!(
                Level::DEBUG,
                message = "Ending poll because of incoming connection"
            );
            Ok(true)
        } else {
            event!(
                Level::DEBUG,
                message = "Ending poll because of timeout expiraton"
            );
            Ok(false)
        }
    }
}
