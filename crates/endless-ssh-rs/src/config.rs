use std::num::{NonZeroU8, NonZeroU16, NonZeroU32};
use std::time::Duration;

use tracing::{Level, event};

pub const DEFAULT_PORT: NonZeroU16 = NonZeroU16::new(2223).unwrap();
pub const DEFAULT_DELAY_MS: NonZeroU32 = NonZeroU32::new(10000).unwrap();
pub const DEFAULT_MAX_LINE_LENGTH: NonZeroU8 = NonZeroU8::new(32).unwrap();
pub const DEFAULT_MAX_CLIENTS: NonZeroU8 = NonZeroU8::new(64).unwrap();

#[derive(Debug, PartialEq, Eq)]
pub struct Config {
    pub bind_family: BindFamily,
    pub delay: Duration,
    pub max_clients: NonZeroU8,
    pub max_line_length: NonZeroU8,
    pub port: NonZeroU16,
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

    pub fn log(&self) {
        event!(Level::INFO, "Port: {}", self.port);
        event!(Level::INFO, "Delay: {}ms", self.delay.as_millis());
        event!(Level::INFO, "MaxLineLength: {}", self.max_line_length);
        event!(Level::INFO, "MaxClients: {}", self.max_clients);
        event!(Level::INFO, "BindFamily: {}", self.bind_family);
    }
}
