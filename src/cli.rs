use crate::config::Config;
use crate::config::DEFAULT_DELAY_MS;
use crate::config::DEFAULT_MAX_CLIENTS;
use crate::config::DEFAULT_MAX_LINE_LENGTH;
use crate::config::DEFAULT_PORT;

use anyhow::Context;

use clap::command;
use clap::value_parser;
use clap::Arg;
use clap::ArgAction;
use clap::ArgMatches;

use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

fn get_cli_matches() -> ArgMatches {
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
                .help("Message millisecond delay")
                .display_order(2)
                .action(ArgAction::Set)
                .default_value(DEFAULT_DELAY_MS.to_string().as_str())
                .value_parser(value_parser!(u64).range(u64::from(1u32)..=u64::from(u32::MAX))),
        )
        .arg(
            Arg::new("max-line-length")
                .short('l')
                .help("Maximum banner line length (3-255)")
                .display_order(4)
                .default_value(DEFAULT_MAX_LINE_LENGTH.to_string().as_str())
                .value_parser(value_parser!(u64).range(3..=255)),
        )
        .arg(
            Arg::new("max-clients")
                .short('m')
                .help("Maximum number of clients")
                .display_order(5)
                .default_value(DEFAULT_MAX_CLIENTS.to_string().as_str())
                .value_parser(value_parser!(u64).range(u64::from(1u32)..=u64::from(u32::MAX))),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .help("Listening port")
                .display_order(6)
                .default_value(DEFAULT_PORT.to_string().as_str())
                .value_parser(value_parser!(u64).range(u64::from(1u16)..=u64::from(u16::MAX))),
        )
        .arg(
            Arg::new("diagnostics")
                .short('v')
                .help("Print diagnostics to standard output (repeatable)")
                .display_order(7)
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("help")
                .short('h')
                .help("Print this help message and exit")
                .display_order(9)
                .action(ArgAction::Help),
        )
        .get_matches()
}

pub(crate) fn parse_cli(config: &mut Config) -> Result<(), anyhow::Error> {
    let matches = get_cli_matches();

    if Some(&true) == matches.get_one("only_4") {
        config.set_bind_family_ipv4();
    } else if Some(&true) == matches.get_one("only_6") {
        config.set_bind_family_ipv6();
    }

    if Some(clap::ValueSource::CommandLine) == matches.value_source("delay") {
        let delay_match: Option<&u64> = matches.get_one("delay");
        if let Some(&d) = delay_match {
            let arg_u32 =
                u32::try_from(d).with_context(|| format!("Couldn't convert '{}' to u32", d))?;

            let non_zero_arg = NonZeroU32::new(arg_u32)
                .with_context(|| format!("{} is not a valid value for delay", arg_u32))?;

            config.set_delay(non_zero_arg);
        }
    }

    if Some(clap::ValueSource::CommandLine) == matches.value_source("port") {
        let port_match: Option<&u64> = matches.get_one("port");
        if let Some(&p) = port_match {
            let arg_u16 =
                u16::try_from(p).with_context(|| format!("Couldn't convert '{}' to u16", p))?;

            let non_zero_arg = NonZeroU16::new(arg_u16)
                .with_context(|| format!("{} is not a valid value for port", arg_u16))?;

            config.set_port(non_zero_arg);
        }
    }

    if Some(clap::ValueSource::CommandLine) == matches.value_source("max-line-length") {
        if let Some(&l) = matches.get_one::<u64>("max-line-length") {
            let arg_usize =
                usize::try_from(l).with_context(|| format!("Couldn't convert '{}' to usize", l))?;

            let non_zero_arg = NonZeroUsize::new(arg_usize).with_context(|| {
                format!("{} is not a valid value for max-line-length", arg_usize)
            })?;

            config.set_max_line_length(non_zero_arg)?;
        }
    }

    if Some(clap::ValueSource::CommandLine) == matches.value_source("max-clients") {
        if let Some(&c) = matches.get_one::<NonZeroUsize>("max-clients") {
            config.set_max_clients(c);
        }
    }

    Ok(())
}
