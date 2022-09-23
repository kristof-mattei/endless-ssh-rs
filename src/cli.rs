use anyhow::Context;
use clap::parser::ValueSource;
use mockall_double::double;
use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use crate::config::Config;
use crate::config::DEFAULT_DELAY_MS;
use crate::config::DEFAULT_MAX_CLIENTS;
use crate::config::DEFAULT_MAX_LINE_LENGTH;
use crate::config::DEFAULT_PORT;

use clap::command;
use clap::value_parser;
use clap::Arg;
use clap::ArgAction;
use clap::Command;
use lazy_static::lazy_static;
use mockall::automock;

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
    // is merged in
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

    if Some(&true) == matches.get_one("only_4") {
        config.set_bind_family_ipv4();
    } else if Some(&true) == matches.get_one("only_6") {
        config.set_bind_family_ipv6();
    }

    if Some(ValueSource::CommandLine) == matches.value_source("delay") {
        let delay_match: Option<&u64> = matches.get_one("delay");
        if let Some(&d) = delay_match {
            let arg_u32 =
                u32::try_from(d).with_context(|| format!("Couldn't convert '{}' to u32", d))?;

            let non_zero_arg = NonZeroU32::new(arg_u32)
                .with_context(|| format!("{} is not a valid value for delay", arg_u32))?;

            config.set_delay(non_zero_arg);
        }
    }

    if Some(ValueSource::CommandLine) == matches.value_source("port") {
        let port_match: Option<&u64> = matches.get_one("port");
        if let Some(&p) = port_match {
            let arg_u16 =
                u16::try_from(p).with_context(|| format!("Couldn't convert '{}' to u16", p))?;

            let non_zero_arg = NonZeroU16::new(arg_u16)
                .with_context(|| format!("{} is not a valid value for port", arg_u16))?;

            config.set_port(non_zero_arg);
        }
    }

    if Some(ValueSource::CommandLine) == matches.value_source("max-line-length") {
        if let Some(&l) = matches.get_one::<u64>("max-line-length") {
            let arg_usize =
                usize::try_from(l).with_context(|| format!("Couldn't convert '{}' to usize", l))?;

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
                usize::try_from(c).with_context(|| format!("Couldn't convert '{}' to usize", c))?;

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
    use std::sync::{Mutex, MutexGuard};

    use mockall::lazy_static;

    use crate::cli::{build_clap_matcher, mock_matches_wrap::get_matches_context};

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

    #[test]
    // #[should_panic]
    fn test_get_matches_1() {
        let _m = get_lock(&MTX);

        // mock cli
        let ctx = get_matches_context();

        // fake input
        let command_line = ["foo", "bar"];

        // let mut result: Option<_> = Option::None;

        // mock
        ctx.expect()
            .returning(move || build_clap_matcher().try_get_matches_from(command_line));

        // let result = parse_cli();

        // assert!(matches!(result, Err(_)));
    }
}
