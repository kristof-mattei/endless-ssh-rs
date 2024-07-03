use std::num::{NonZeroU16, NonZeroU32, NonZeroUsize};
use std::time::Duration;

use tracing::{event, Level};

pub(crate) const DEFAULT_PORT: NonZeroU16 = unsafe { NonZeroU16::new_unchecked(2223) };
pub(crate) const DEFAULT_DELAY_MS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(10000) };
pub(crate) const DEFAULT_MAX_LINE_LENGTH: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(32) };
pub(crate) const DEFAULT_MAX_CLIENTS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(64) };

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub(crate) port: NonZeroU16,
    pub(crate) delay: Duration,
    pub(crate) max_line_length: NonZeroUsize,
    pub(crate) max_clients: NonZeroUsize,
    pub(crate) bind_family: BindFamily,
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum BindFamily {
    Ipv4,
    #[allow(dead_code)]
    Ipv6,
    DualStack,
}

impl std::fmt::Display for BindFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindFamily::Ipv4 => write!(f, "IPv4"),
            BindFamily::Ipv6 => write!(f, "IPv6"),
            BindFamily::DualStack => write!(f, "Dual Stack"),
        }
    }
}

impl Config {
    pub(crate) fn new() -> Self {
        Self {
            port: DEFAULT_PORT,
            delay: Duration::from_millis(DEFAULT_DELAY_MS.get().into()),
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
            max_clients: DEFAULT_MAX_CLIENTS,
            bind_family: BindFamily::DualStack,
        }
    }

    pub(crate) fn set_port(&mut self, port: NonZeroU16) {
        self.port = port;
    }

    pub(crate) fn set_delay(&mut self, delay: NonZeroU32) {
        self.delay = Duration::from_millis(u64::from(delay.get()));
    }

    pub(crate) fn set_max_clients(&mut self, max_clients: NonZeroUsize) {
        self.max_clients = max_clients;
    }

    pub(crate) fn set_max_line_length(&mut self, l: NonZeroUsize) {
        self.max_line_length = l;
    }

    pub(crate) fn set_bind_family_ipv4_only(&mut self) {
        self.bind_family = BindFamily::Ipv4;
    }

    pub(crate) fn set_bind_family_dual_stack(&mut self) {
        self.bind_family = BindFamily::DualStack;
    }

    pub(crate) fn set_bind_family_ipv6_only(&mut self) {
        self.bind_family = BindFamily::Ipv6;
    }

    pub(crate) fn log(&self) {
        event!(Level::INFO, "Port: {}", self.port);
        event!(Level::INFO, "Delay: {}ms", self.delay.as_millis());
        event!(Level::INFO, "MaxLineLength: {}", self.max_line_length);
        event!(Level::INFO, "MaxClients: {}", self.max_clients);
        event!(Level::INFO, "BindFamily: {}", self.bind_family);
    }
}
