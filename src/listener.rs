use crate::config::Config;

use anyhow::Context;
use anyhow::Result;

use libc::poll;
use libc::pollfd;
use libc::POLLIN;

use tracing::event;
use tracing::instrument;
use tracing::Level;

use std::fmt::Display;
use std::io::Error;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use std::net::TcpListener;
use std::ops::Deref;
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of_mut;
use time::Duration;

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
        match self {
            Timeout::Infinite => -1,
            Timeout::Duration(m) => i32::try_from(m.whole_milliseconds()).unwrap_or(i32::MAX),
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
        let listener = match config.bind_family {
            IpAddr::V4(a) => {
                let sa: SocketAddrV4 = SocketAddrV4::new(a, config.port.get());

                TcpListener::bind(sa).unwrap()
            },
            IpAddr::V6(a) => {
                let sa: SocketAddrV6 = SocketAddrV6::new(a, config.port.get(), 0, 0);

                TcpListener::bind(sa).unwrap()
            },
        };

        listener
            .set_nonblocking(true)
            .context("Failed to set listener to non-blocking")?;

        event!(Level::DEBUG, message = "Bound and listening!", ?listener);

        Ok(Self(listener))
    }

    #[instrument(skip(self), fields(self = ?self.0, timeout = ?timeout))]
    pub(crate) fn wait_poll(
        &self,
        can_accept_more_clients: bool,
        timeout: Timeout,
    ) -> Result<bool, anyhow::Error> {
        // Wait for next event
        let mut fds: pollfd = pollfd {
            fd: self.0.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        if can_accept_more_clients {
            event!(Level::DEBUG, message = "Polling socket...");
        } else {
            event!(
                Level::DEBUG,
                message = "Maximum clients reached, just waiting until timeout expires"
            );
        }

        let r = unsafe {
            poll(
                addr_of_mut!(fds),
                if can_accept_more_clients { 1 } else { 0 },
                timeout.as_c_timeout(),
            )
        };

        if r == -1 {
            let last_error = Error::last_os_error();
            // poll & ppoll's EINTR cannot be avoided by using SA_RESTART
            // see https://stackoverflow.com/a/48553220
            if ErrorKind::Interrupted == last_error.kind() {
                event!(Level::DEBUG, "Poll interrupted, but that's ok");
                return Ok(false);
            }

            let wrapped = anyhow::Error::new(last_error)
                .context("Something went wrong during polling / waiting for the next call");

            event!(Level::ERROR, ?wrapped);

            return Err(wrapped);
        }

        if fds.revents & POLLIN == POLLIN {
            event!(
                Level::INFO,
                message = "Ending poll because of incoming connection"
            );
            Ok(true)
        } else {
            event!(
                Level::INFO,
                message = "Ending poll because of timeout expiraton"
            );
            Ok(false)
        }
    }
}
