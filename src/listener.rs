use anyhow::{Context, Result};
use libc::poll;
use libc::pollfd;
use libc::POLLIN;
use std::io::Error;
use std::io::ErrorKind;
use std::net::IpAddr;
use std::net::SocketAddrV4;
use std::net::SocketAddrV6;
use std::net::TcpListener;
use std::ops::Deref;
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of_mut;

use crate::config::Config;

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

        Ok(Self(listener))
    }

    pub(crate) fn wait_poll(&self, timeout: i32) -> Result<bool, anyhow::Error> {
        // Wait for next event
        let mut fds: pollfd = pollfd {
            fd: self.0.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        };

        println!("timoeut: {}", timeout);

        if unsafe { poll(addr_of_mut!(fds), 1, timeout) } == -1 {
            let last_error = Error::last_os_error();
            // poll & ppoll's EINTR cannot be avoided by using SA_RESTART
            // see https://stackoverflow.com/a/48553220
            if ErrorKind::Interrupted == last_error.kind() {
                return Ok(false);
            }

            return Err(last_error)
                .with_context(|| "Something went wrong while waiting for the next call");
        }

        Ok(fds.revents & POLLIN == POLLIN)
    }
}
