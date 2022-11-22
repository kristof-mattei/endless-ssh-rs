use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use anyhow::Context;
use clap::command;
use clap::parser::ValueSource;
use clap::value_parser;
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use lazy_static::lazy_static;
use mockall::automock;
use mockall_double::double;
use tracing::event;
use tracing::Level;

use crate::config::Config;
use crate::config::DEFAULT_DELAY_MS;
use crate::config::DEFAULT_MAX_CLIENTS;
use crate::config::DEFAULT_MAX_LINE_LENGTH;
use crate::config::DEFAULT_PORT;

lazy_static! {
    static ref DEFAULT_PORT_VALUE: String = DEFAULT_PORT.to_string();
    static ref DEFAULT_MAX_CLIENTS_VALUE: String = DEFAULT_MAX_CLIENTS.to_string();
    static ref DEFAULT_DELAY_MS_VALUE: String = DEFAULT_DELAY_MS.to_string();
    static ref DEFAULT_MAX_LINE_LENGTH_VALUE: String = DEFAULT_MAX_LINE_LENGTH.to_string();
}

fn build_clap_matcher() -> Command {
    command!()
        .disable_help_flag(true)
        .arg(
            Arg::new("only_4")
                .short('4')
                .help("Bind to IPv4 only")
                .group("ip_version")
                .display_order(0)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("only_6")
                .short('6')
                .help("Bind to IPv6 only")
                .group("ip_version")
                .display_order(1)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("delay")
                .short('d')
                .long("delay")
                .help("Message millisecond delay")
                .display_order(2)
                .action(ArgAction::Set)
                .default_value(DEFAULT_DELAY_MS_VALUE.as_str())
                .value_parser(
                    value_parser!(u64).range(u64::from(1u32)..=u64::try_from(i32::MAX).unwrap()),
                ),
        )
        .arg(
            Arg::new("max-line-length")
                .short('l')
                .long("max-line-length")
                .help("Maximum banner line length (3-255)")
                .display_order(4)
                .default_value(DEFAULT_MAX_LINE_LENGTH_VALUE.as_str())
                .value_parser(value_parser!(u64).range(3..=255)),
        )
        .arg(
            Arg::new("max-clients")
                .short('m')
                .long("max-clients")
                .help("Maximum number of clients")
                .display_order(5)
                .default_value(DEFAULT_MAX_CLIENTS_VALUE.as_str())
                .value_parser(value_parser!(u64).range(u64::from(1u32)..=u64::from(u32::MAX))),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .help("Listening port")
                .display_order(6)
                .default_value(DEFAULT_PORT_VALUE.as_str())
                .value_parser(value_parser!(u64).range(u64::from(1u16)..=u64::from(u16::MAX))),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .help("Print this help message and exit")
                .display_order(9)
                .action(ArgAction::Help),
        )
}

#[automock]
mod matches_wrap {

    use super::build_clap_matcher;

    #[cfg_attr(test, allow(dead_code))]
    // delete when https://github.com/rust-lang/rust-clippy/pull/9486
    // is in main
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn get_matches() -> std::result::Result<clap::ArgMatches, clap::Error> {
        let matcher = build_clap_matcher();

        matcher.try_get_matches()
    }
}

#[double]
use self::matches_wrap as matches;

pub(crate) fn parse_cli() -> Result<Config, anyhow::Error> {
    let matches = matches::get_matches()?;

    let mut config = Config::new();

    match (
        matches.get_one("only_4").unwrap_or(&false),
        matches.get_one("only_6").unwrap_or(&false),
    ) {
        (true, false) => {
            config.set_bind_family_ipv4_only();
        },
        (false, true) => {
            config.set_bind_family_ipv6_only();
            event!(Level::WARN, "Ipv6 only currently implies dual stack");
        },
        _ => {
            config.set_bind_family_dual_stack();
        },
    }

    if Some(ValueSource::CommandLine) == matches.value_source("delay") {
        let delay_match: Option<&u64> = matches.get_one("delay");
        if let Some(&d) = delay_match {
            let arg_u32 =
                u32::try_from(d).with_context(|| format!("Couldn't convert '{d}' to u32"))?;

            let non_zero_arg = NonZeroU32::new(arg_u32)
                .with_context(|| format!("{arg_u32} is not a valid value for delay"))?;

            config.set_delay(non_zero_arg);
        }
    }

    if Some(ValueSource::CommandLine) == matches.value_source("port") {
        let port_match: Option<&u64> = matches.get_one("port");
        if let Some(&p) = port_match {
            let arg_u16 =
                u16::try_from(p).with_context(|| format!("Couldn't convert '{p}' to u16"))?;

            let non_zero_arg = NonZeroU16::new(arg_u16)
                .with_context(|| format!("{arg_u16} is not a valid value for port"))?;

            config.set_port(non_zero_arg);
        }
    }

    if Some(ValueSource::CommandLine) == matches.value_source("max-line-length") {
        if let Some(&l) = matches.get_one::<u64>("max-line-length") {
            let arg_usize =
                usize::try_from(l).with_context(|| format!("Couldn't convert '{l}' to usize"))?;

            let non_zero_arg = NonZeroUsize::try_from(arg_usize).map_err(|_| {
                anyhow::Error::msg(format!(
                    "{} is not a valid value for max-line-length",
                    arg_usize
                ))
            })?;

            config.set_max_line_length(non_zero_arg);
        }
    }

    if Some(ValueSource::CommandLine) == matches.value_source("max-clients") {
        if let Some(&c) = matches.get_one::<u64>("max-clients") {
            let arg_usize =
                usize::try_from(c).with_context(|| format!("Couldn't convert '{c}' to usize"))?;

            let non_zero_arg = NonZeroUsize::try_from(arg_usize).map_err(|_| {
                anyhow::Error::msg(format!(
                    "{} is not a valid value for max-clients",
                    arg_usize
                ))
            })?;

            config.set_max_clients(non_zero_arg);
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::{
        num::{NonZeroU16, NonZeroUsize},
        sync::{Mutex, MutexGuard},
    };

    use mockall::lazy_static;

    use crate::{
        cli::{build_clap_matcher, mock_matches_wrap::get_matches_context, parse_cli},
        config::{BindFamily, Config},
    };

    lazy_static! {
        static ref MTX: Mutex<()> = Mutex::new(());
    }

    // When a test panics, it will poison the Mutex. Since we don't actually
    // care about the state of the data we ignore that it is poisoned and grab
    // the lock regardless.  If you just do `let _m = &MTX.lock().unwrap()`, one
    // test panicking will cause all other tests that try and acquire a lock on
    // that Mutex to also panic.
    fn get_lock(m: &'static Mutex<()>) -> MutexGuard<'static, ()> {
        match m.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn parse_factory(input: &'static str) -> Result<Config, anyhow::Error> {
        let _m = get_lock(&MTX);

        // mock cli
        let ctx = get_matches_context();

        // fake input
        let command_line = input.split_whitespace().collect::<Vec<&str>>();

        // mock
        ctx.expect()
            .returning_st(move || build_clap_matcher().try_get_matches_from(&command_line));

        parse_cli()
    }

    #[test]
    fn bad_cli_options_1() {
        let result = parse_factory("foo bar");

        assert!(result.is_err());
    }

    #[test]
    fn bad_cli_options_2() {
        let result = parse_factory("endless-ssh-rs bar");

        assert!(result.is_err());
    }

    #[test]
    fn parses_port() {
        let result = parse_factory("endless-ssh-rs --port 2000");

        let expected_config = Config {
            port: NonZeroU16::new(2000).unwrap(),
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_delay() {
        let result = parse_factory("endless-ssh-rs --delay 100");

        let expected_config = Config {
            delay: std::time::Duration::from_millis(100),
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_max_clients() {
        let result = parse_factory("endless-ssh-rs --max-clients 50");

        let expected_config = Config {
            max_clients: NonZeroUsize::new(50).unwrap(),
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_max_line_length() {
        let result = parse_factory("endless-ssh-rs --max-line-length 70");

        let expected_config = Config {
            max_line_length: NonZeroUsize::new(70).unwrap(),
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn ensures_minimum_line_length() {
        let result = parse_factory("endless-ssh-rs --max-line-length 2");

        assert!(result.is_err());
    }

    #[test]
    fn parses_ipv4_only() {
        let result = parse_factory("endless-ssh-rs -4");

        let expected_config = Config {
            bind_family: BindFamily::Ipv4,
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn parses_ipv6_only() {
        let result = parse_factory("endless-ssh-rs -6");

        let expected_config = Config {
            bind_family: BindFamily::Ipv6,
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }

    #[test]
    fn no_ip_options_mean_dual_stack() {
        let result = parse_factory("endless-ssh-rs");

        let expected_config = Config {
            bind_family: BindFamily::DualStack,
            ..Default::default()
        };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_config);
    }
    #[test]
    fn specifying_ipv4_and_ipv6_throw_error() {
        let result = parse_factory("endless-ssh-rs -4 -6");

        assert!(result.is_err());
    }
}
