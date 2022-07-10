use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use crate::log::logmsg;
use crate::log::LogLevel;

pub(crate) const DEFAULT_PORT: u16 = 2223; // 1 -> 65535

// milliseconds
pub(crate) const DEFAULT_DELAY: u32 = 400;
pub(crate) const DEFAULT_MAX_LINE_LENGTH: u64 = 32;
pub(crate) const DEFAULT_MAX_CLIENTS: u64 = 4096;

pub(crate) struct Config {
    pub(crate) port: NonZeroU16,
    pub(crate) delay: NonZeroU32,
    pub(crate) max_line_length: NonZeroUsize,
    pub(crate) max_clients: NonZeroUsize,
    pub(crate) bind_family: IpAddr,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: DEFAULT_PORT.try_into().expect("Default port cannot be 0"),
            delay: DEFAULT_DELAY.try_into().expect("Default delay cannot be 0"),
            max_line_length: usize::try_from(DEFAULT_MAX_LINE_LENGTH)
                .expect("Default max line length should fit a usize")
                .try_into()
                .expect("Default max line length cannot be 0"),
            max_clients: usize::try_from(DEFAULT_MAX_CLIENTS)
                .expect("Default max clients should fit a usize")
                .try_into()
                .expect("Default max clients cannot be 0"),
            bind_family: IpAddr::V6(Ipv6Addr::UNSPECIFIED),
        }
    }
}

impl Config {
    pub(crate) fn set_port(&mut self, port: NonZeroU16) {
        self.port = port;
    }

    pub(crate) fn set_delay(&mut self, delay: NonZeroU32) {
        self.delay = delay;
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

    // pub(crate) fn set_bind_family_unspecified(&mut self) -> Result<(), ()> {
    //     self.bind_family = AF_UNSPEC;
    //     Ok(())
    // }

    pub(crate) fn log(&self) {
        logmsg(LogLevel::Info, format!("Port {}", self.port));
        logmsg(LogLevel::Info, format!("Delay {}", self.delay));
        logmsg(
            LogLevel::Info,
            format!("MaxLineLength {}", self.max_line_length),
        );
        logmsg(LogLevel::Info, format!("MaxClients {}", self.max_clients));
        let bind_family_description = match self.bind_family {
            IpAddr::V6(_) => "Ipv6 Only",
            IpAddr::V4(_) => "Ipv4 Only",
        };
        logmsg(
            LogLevel::Info,
            format!("BindFamily {}", bind_family_description),
        );
    }
}
