use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use tracing::event;
use tracing::Level;

pub(crate) const DEFAULT_PORT: NonZeroU16 = unsafe { NonZeroU16::new_unchecked(2223) };
pub(crate) const DEFAULT_DELAY_MS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(10000) };
pub(crate) const DEFAULT_MAX_LINE_LENGTH: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(32) };
pub(crate) const DEFAULT_MAX_CLIENTS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(64) };

pub(crate) struct Config {
    pub(crate) port: NonZeroU16,
    pub(crate) delay_ms: NonZeroU32,
    pub(crate) max_line_length: NonZeroUsize,
    pub(crate) max_clients: NonZeroUsize,
    pub(crate) bind_family: IpAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT,
            delay_ms: DEFAULT_DELAY_MS,
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
            max_clients: DEFAULT_MAX_CLIENTS,
            bind_family: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        }
    }
}

impl Config {
    pub(crate) fn set_port(&mut self, port: NonZeroU16) {
        self.port = port;
    }

    pub(crate) fn set_delay(&mut self, delay: NonZeroU32) {
        self.delay_ms = delay;
    }

    pub(crate) fn set_max_clients(&mut self, max_clients: NonZeroUsize) {
        self.max_clients = max_clients;
    }

    pub(crate) fn set_max_line_length(&mut self, l: NonZeroUsize) -> Result<(), anyhow::Error> {
        if l.get() < 3 || l.get() > 255 {
            Err(anyhow::Error::msg(format!(
                "Invalid maximum line length: {}",
                l.get()
            )))
        } else {
            self.max_line_length = l;
            Ok(())
        }
    }

    pub(crate) fn set_bind_family_ipv4(&mut self) {
        self.bind_family = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
    }

    pub(crate) fn set_bind_family_ipv6(&mut self) {
        self.bind_family = IpAddr::V6(Ipv6Addr::UNSPECIFIED);
    }

    pub(crate) fn log(&self) {
        event!(Level::INFO, "Port: {}", self.port);
        event!(Level::INFO, "Delay: {}ms", self.delay_ms);
        event!(Level::INFO, "MaxLineLength: {}", self.max_line_length);
        event!(Level::INFO, "MaxClients: {}", self.max_clients);
        let bind_family_description = match self.bind_family {
            IpAddr::V6(_) => "Ipv6 Only",
            IpAddr::V4(_) => "Ipv4 Only",
        };
        event!(Level::INFO, "BindFamily: {}", bind_family_description);
    }
}
