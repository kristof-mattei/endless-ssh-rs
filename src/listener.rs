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

#[derive(Debug)]
pub(crate) enum WaitFor {
    Infinite,
    Milliseconds(i32),
}

impl WaitFor {
    pub(crate) fn new(milliseconds: i32) -> WaitFor {
        if milliseconds < 0 {
            WaitFor::Infinite
        } else {
            WaitFor::Milliseconds(milliseconds)
        }
    }
}

impl Display for WaitFor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", i32::from(self)))
    }
}

impl From<i32> for WaitFor {
    fn from(milliseconds: i32) -> Self {
        WaitFor::new(milliseconds)
    }
}

impl From<&WaitFor> for i32 {
    fn from(wait_for: &WaitFor) -> Self {
        match wait_for {
            WaitFor::Infinite => -1,
            WaitFor::Milliseconds(m) => *m,
        }
    }
}

impl From<WaitFor> for i32 {
    fn from(wait_for: WaitFor) -> Self {
        match wait_for {
            WaitFor::Infinite => -1,
            WaitFor::Milliseconds(m) => m,
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
            .with_context(|| "Failed to set listener to non-blocking")?;

        event!(Level::DEBUG, ?listener, "Bound and listening!");

        Ok(Self(listener))
    }

    #[instrument]
    pub(crate) fn wait_poll(&self, timeout: WaitFor) -> Result<bool, anyhow::Error> {
        // Wait for next event
        let mut fds: pollfd = pollfd {
            fd: self.0.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        event!(Level::DEBUG, "poll({}, {})", 1, timeout);

        let r = unsafe { poll(addr_of_mut!(fds), 1, timeout.into()) };

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
            event!(Level::INFO, "Done polling because of incoming data");
            Ok(true)
        } else {
            event!(Level::INFO, "Done polling because of timeout expiration");
            Ok(false)
        }
    }
}
