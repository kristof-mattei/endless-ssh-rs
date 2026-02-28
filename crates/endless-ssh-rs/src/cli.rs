use std::env;
use std::ffi::OsString;
use std::num::{NonZeroU8, NonZeroU16};
use std::time::Duration;

use clap::error::ErrorKind;
use clap::{ArgAction, Parser, value_parser};
use color_eyre::eyre;
use tracing::{Level, event};

use crate::config::{
    BindFamily, Config, DEFAULT_DELAY_MS, DEFAULT_MAX_CLIENTS, DEFAULT_MAX_LINE_LENGTH,
    DEFAULT_PORT,
};

fn delay_parser(value: &str) -> Result<Duration, clap::Error> {
    let timeout_ms = value
        .parse()
        .map_err(|_| clap::Error::new(ErrorKind::ValueValidation))?;

    Ok(Duration::from_millis(timeout_ms))
}

#[derive(Debug, Parser)]
#[command(disable_help_flag = true)]
pub struct Cli {
    #[clap(
        short = '4',
        long = "only_4",
        action = ArgAction::SetTrue,
        help = "Bind to IPv4 only",
        group = "ip_version"
    )]
    only_4: bool,

    #[clap(
        short = '6',
        long = "only_6",
        action = ArgAction::SetTrue,
        help = "Bind to IPv6 only",
        group = "ip_version"
    )]
    only_6: bool,

    #[clap(
        short = 'd',
        long = "delay",
        default_value = DEFAULT_DELAY_MS.to_string(),
        help = "Message millisecond delay",
        value_parser = delay_parser
    )]
    delay: Duration,

    #[clap(
        short = 'l',
        long = "max-line-length",
        default_value_t = DEFAULT_MAX_LINE_LENGTH.get(),
        help = "Maximum banner line length (3-255)",
        value_parser = value_parser!(u8).range(3..=255)
    )]
    max_line_length: u8,

    #[clap(
        short = 'm',
        long = "max-clients",
        default_value_t = DEFAULT_MAX_CLIENTS.get(),
        help = "Maximum number of clients",
        value_parser = value_parser!(u8).range(1..)
    )]
    max_clients: u8,

    #[clap(
        short = 'p',
        long = "port",
        default_value_t = DEFAULT_PORT.get(),
        help = "Listening port",
        value_parser = value_parser!(u16).range(1..)
    )]
    port: u16,

    #[clap(
        short = 'h',
        long = "help",
        help = "Print this help message and exit",
        action = ArgAction::Help,
    )]
    help: (),
}

impl From<Cli> for Config {
    fn from(matches: Cli) -> Self {
        let bind_family = match (matches.only_4, matches.only_6) {
            (true, false) => BindFamily::Ipv4,
            (false, true) => {
                event!(Level::WARN, "Ipv6 only currently implies dual stack");

                BindFamily::Ipv6
            },
            (false, false) => BindFamily::DualStack,
            (true, true) => unreachable!("Guaranteed by clap"),
        };

        Config {
            bind_family,
            delay: matches.delay,
            max_clients: NonZeroU8::new(matches.max_clients).expect("Guaranteed by clap"),
            max_line_length: NonZeroU8::new(matches.max_line_length).expect("Guaranteed by clap"),
            port: NonZeroU16::new(matches.port).expect("Guaranteed by clap"),
        }
    }
}

pub fn parse_cli() -> Result<Config, eyre::Error> {
    parse_cli_from(env::args_os())
}

fn parse_cli_from<I, T>(from: I) -> Result<Config, eyre::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Ok(Cli::try_parse_from(from)?.into())
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU8, NonZeroU16};

    use color_eyre::eyre;
    use pretty_assertions::assert_eq;

    use super::parse_cli_from;
    use crate::config::{BindFamily, Config};

    fn parse_factory(input: &'static str) -> Result<Config, eyre::Report> {
        // fake input
        let command_line = input.split_whitespace().collect::<Vec<&str>>();

        parse_cli_from(command_line)
    }

    #[test]
    fn bad_cli_options_1() {
        let result = parse_factory("foo bar");

        #[expect(unused_must_use, reason = "Testing")]
        result.unwrap_err();
    }

    #[test]
    fn bad_cli_options_2() {
        let result = parse_factory("endless-ssh-rs bar");

        #[expect(unused_must_use, reason = "Testing")]
        result.unwrap_err();
    }

    #[test]
    fn parses_port() {
        let result = parse_factory("endless-ssh-rs --port 2000");

        let expected_config = Config {
            port: NonZeroU16::new(2000).unwrap(),
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_delay() {
        let result = parse_factory("endless-ssh-rs --delay 100");

        let expected_config = Config {
            delay: std::time::Duration::from_millis(100),
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_max_clients() {
        let result = parse_factory("endless-ssh-rs --max-clients 50");

        let expected_config = Config {
            max_clients: NonZeroU8::new(50).unwrap(),
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_max_line_length() {
        let result = parse_factory("endless-ssh-rs --max-line-length 70");

        let expected_config = Config {
            max_line_length: NonZeroU8::new(70).unwrap(),
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn ensures_minimum_line_length() {
        let result = parse_factory("endless-ssh-rs --max-line-length 2");

        #[expect(unused_must_use, reason = "Testing")]
        result.unwrap_err();
    }

    #[test]
    fn parses_ipv4_only() {
        let result = parse_factory("endless-ssh-rs -4");

        let expected_config = Config {
            bind_family: BindFamily::Ipv4,
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_ipv6_only() {
        let result = parse_factory("endless-ssh-rs -6");

        let expected_config = Config {
            bind_family: BindFamily::Ipv6,
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn no_ip_options_mean_dual_stack() {
        let result = parse_factory("endless-ssh-rs");

        let expected_config = Config {
            bind_family: BindFamily::DualStack,
            ..Config::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }
    #[test]
    fn specifying_ipv4_and_ipv6_throw_error() {
        let result = parse_factory("endless-ssh-rs -4 -6");

        #[expect(unused_must_use, reason = "Testing")]
        result.unwrap_err();
    }
}
