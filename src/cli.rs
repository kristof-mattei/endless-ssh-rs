use std::env;
use std::ffi::OsString;
use std::num::{NonZeroU16, NonZeroU32, NonZeroUsize};

use clap::parser::ValueSource;
use clap::{command, value_parser, Arg, ArgAction, Command};
use color_eyre::eyre::{self, WrapErr};
use lazy_static::lazy_static;
use tracing::{event, Level};

use crate::config::{
    Config, DEFAULT_DELAY_MS, DEFAULT_MAX_CLIENTS, DEFAULT_MAX_LINE_LENGTH, DEFAULT_PORT,
};

lazy_static! {
    static ref DEFAULT_PORT_VALUE: String = DEFAULT_PORT.to_string();
    static ref DEFAULT_MAX_CLIENTS_VALUE: String = DEFAULT_MAX_CLIENTS.to_string();
    static ref DEFAULT_DELAY_MS_VALUE: String = DEFAULT_DELAY_MS.to_string();
    static ref DEFAULT_MAX_LINE_LENGTH_VALUE: String = DEFAULT_MAX_LINE_LENGTH.to_string();
}

fn build_clap_matcher() -> Command {
    command!()
        .disable_help_flag(true)
        .color(clap::ColorChoice::Always)
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

pub(crate) fn parse_cli() -> Result<Config, eyre::Error> {
    parse_cli_from(env::args_os())
}

fn parse_cli_from<I, T>(from: I) -> Result<Config, eyre::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let matches = build_clap_matcher().try_get_matches_from(from)?;

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

    if let Some(&d) = get_user_cli_value::<u64>(&matches, "delay") {
        let arg_u32 =
            u32::try_from(d).wrap_err_with(|| format!("Couldn't convert '{}' to u32", d))?;

        let non_zero_arg = NonZeroU32::try_from(arg_u32)
            .wrap_err_with(|| format!("{} is not a valid value for delay", arg_u32))?;

        config.set_delay(non_zero_arg);
    }

    if let Some(&p) = get_user_cli_value::<u64>(&matches, "port") {
        let arg_u16 =
            u16::try_from(p).wrap_err_with(|| format!("Couldn't convert '{}' to u16", p))?;

        let non_zero_arg = NonZeroU16::try_from(arg_u16)
            .wrap_err_with(|| format!("{} is not a valid value for port", arg_u16))?;

        config.set_port(non_zero_arg);
    }

    if let Some(&l) = get_user_cli_value::<u64>(&matches, "max-line-length") {
        let arg_usize =
            usize::try_from(l).wrap_err_with(|| format!("Couldn't convert '{}' to usize", l))?;

        let non_zero_arg = NonZeroUsize::try_from(arg_usize)
            .wrap_err_with(|| format!("{} is not a valid value for max-line-length", arg_usize))?;

        config.set_max_line_length(non_zero_arg);
    }

    if let Some(&c) = get_user_cli_value::<u64>(&matches, "max-clients") {
        let arg_usize =
            usize::try_from(c).wrap_err_with(|| format!("Couldn't convert '{}' to usize", c))?;

        let non_zero_arg = NonZeroUsize::try_from(arg_usize)
            .wrap_err_with(|| format!("{} is not a valid value for max-clients", arg_usize))?;

        config.set_max_clients(non_zero_arg);
    }

    Ok(config)
}

fn get_user_cli_value<'a, T>(matches: &'a clap::ArgMatches, key: &str) -> Option<&'a T>
where
    T: Clone + Send + Sync + 'static,
{
    // our CLI has defaults, so we check if the user has provided a value
    let Some(ValueSource::CommandLine) = matches.value_source(key) else {
        return None;
    };

    // NOTE: we might change this later to always use the user's input, as we might want this module
    // to drive the config's defaults.
    // I am always confused as to who should do what. Who provides defaults? Who provides upper and lower limits?
    // Because not everything comes through a CLI. I would love to share this with something like
    // a yaml file. But then we run into issues with valid values for a type (say 1 for max-line-length) but
    // that's an invalid number in our logic.
    // on the other hand there are port 100000 which doesn't even fit into our data type

    // return the value provided by the user
    matches.get_one::<T>(key)
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU16, NonZeroUsize};

    use color_eyre::eyre;

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
