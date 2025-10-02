use std::num::{NonZeroU16, NonZeroU32, NonZeroUsize};
use std::time::Duration;

use tracing::{Level, event};

pub const DEFAULT_PORT: NonZeroU16 = NonZeroU16::new(2223).unwrap();
pub const DEFAULT_DELAY_MS: NonZeroU32 = NonZeroU32::new(10000).unwrap();
pub const DEFAULT_MAX_LINE_LENGTH: NonZeroUsize = NonZeroUsize::new(32).unwrap();
pub const DEFAULT_MAX_CLIENTS: NonZeroUsize = NonZeroUsize::new(64).unwrap();

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub port: NonZeroU16,
    pub delay: Duration,
    pub max_line_length: NonZeroUsize,
    pub max_clients: NonZeroUsize,
    pub bind_family: BindFamily,
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum BindFamily {
    Ipv4,
    Ipv6,
    DualStack,
}

impl std::fmt::Display for BindFamily {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            BindFamily::Ipv4 => write!(f, "IPv4"),
            BindFamily::Ipv6 => write!(f, "IPv6"),
            BindFamily::DualStack => write!(f, "Dual Stack"),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        Self {
            port: DEFAULT_PORT,
            delay: Duration::from_millis(DEFAULT_DELAY_MS.get().into()),
            max_line_length: DEFAULT_MAX_LINE_LENGTH,
            max_clients: DEFAULT_MAX_CLIENTS,
            bind_family: BindFamily::DualStack,
        }
    }

    pub fn set_port(&mut self, port: NonZeroU16) {
        self.port = port;
    }

    pub fn set_delay(&mut self, delay: NonZeroU32) {
        self.delay = Duration::from_millis(delay.get().into());
    }

    pub fn set_max_clients(&mut self, max_clients: NonZeroUsize) {
        self.max_clients = max_clients;
    }

    pub fn set_max_line_length(&mut self, l: NonZeroUsize) {
        self.max_line_length = l;
    }

    pub fn set_bind_family_ipv4_only(&mut self) {
        self.bind_family = BindFamily::Ipv4;
    }

    pub fn set_bind_family_dual_stack(&mut self) {
        self.bind_family = BindFamily::DualStack;
    }

    pub fn set_bind_family_ipv6_only(&mut self) {
        self.bind_family = BindFamily::Ipv6;
    }

    pub fn log(&self) {
        event!(Level::INFO, "Port: {}", self.port);
        event!(Level::INFO, "Delay: {}ms", self.delay.as_millis());
        event!(Level::INFO, "MaxLineLength: {}", self.max_line_length);
        event!(Level::INFO, "MaxClients: {}", self.max_clients);
        event!(Level::INFO, "BindFamily: {}", self.bind_family);
    }
}
