use crate::config::Config;
use crate::config::DEFAULT_CONFIG_FILE;
use crate::config::DEFAULT_DELAY;
use crate::config::DEFAULT_MAX_CLIENTS;
use crate::config::DEFAULT_MAX_LINE_LENGTH;
use crate::config::DEFAULT_PORT;
use crate::log::LogLevel;

use clap::value_parser;
use clap::ArgAction;
use clap::Parser;

#[derive(Parser)]
#[allow(clippy::struct_excessive_bools)]
#[clap(
    usage = "Usage: endless-ssh-rs [-vh] [-46] [-d MS] [-f CONFIG] [-l LEN] [-m LIMIT] [-p PORT]"
)]
pub(crate) struct Cli {
    #[clap(
        short = '4',
        help = "Bind to IPv4 only",
        group = "ip_version",
        display_order = 0
    )]
    only_4: bool,

    #[clap(
        short = '6',
        help = "Bind to IPv6 only",
        group = "ip_version",
        display_order = 1
    )]
    only_6: bool,

    #[clap(short = 'd', help="Message millisecond delay",default_value_t=DEFAULT_DELAY, display_order=2)]
    delay: u32,

    #[clap(short = 'f', help="Set and load config file", default_value=DEFAULT_CONFIG_FILE, display_order=3)]
    file: String,

    #[clap(short = 'l', help="Maximum banner line length (3-255)", default_value_t=DEFAULT_MAX_LINE_LENGTH, value_parser=value_parser!(u64).range(3..=255), display_order=4)]
    line_length: u64,

    #[clap(short = 'm', help="Maximum number of clients", default_value_t=DEFAULT_MAX_CLIENTS, value_parser=value_parser!(u64).range(1..=u64::from(u32::MAX)), display_order=5)]
    max_clients: u64,

    #[clap(short = 'p', help="Listening port", default_value_t=DEFAULT_PORT, display_order=6)]
    port: u16,

    #[clap(short = 'v', help="Print diagnostics to standard output (repeatable)",  display_order=7, value_parser=value_parser!(u8).range(LogLevel::None as i64..=LogLevel::Debug as i64), action=ArgAction::Count)]
    verbosity: LogLevel,

    #[clap(
        short = 'V',
        help = "Print version information and exit",
        display_order = 8
    )]
    show_version: bool,

    #[clap(
        short = 'h',
        help = "Print this help message and exit",
        display_order = 9
    )]
    show_help: bool,
}

pub(crate) fn parse_cli_params(config: &mut Config) -> Result<(), ()> {
    let matches = Cli::parse();

    println!("{}", config.port);

    // if true == matches.only_4 {
    //     config.set_bind_family_ipv4();
    // }

    // if true == matches.only_6 {
    //     config.set_bind_family_ipv6();
    // }

    // if let Some(d) = matches.delay {
    //     config.set_delay(d);
    // }

    // if let Some(&p) = matches.port {
    //     config.set_port(p);
    // }
    // //         F_LOWER => {
    // //             //     config_file = optarg;
    // //             // #if defined(__OpenBSD__)
    // //             //                 unveil(config_file, "r");
    // //             //                 if (unveil(0, 0) == -1)
    // //             //                     die();
    // //             // #endif

    // //             //                 config_load(&config, optarg, 1);
    // //         },
    // //         H_LOWER => {
    // //             //                 usage(stdout);
    // //             //                 exit(EXIT_SUCCESS);
    // //         },

    // if let Some(&l) = matches.get_one::<NonZeroUsize>("max-line-length") {
    //     config.set_max_line_length(l);
    // }

    // if let Some(&c) = matches.get_one::<NonZeroUsize>("max-clients") {
    //     config.set_max_clients(c);
    // }

    // //         S_LOWER => {
    // //             //                 logmsg = logsyslog;
    // //         },
    // //         V_LOWER => {
    // //             //                 if (loglevel < log_debug)
    // //             //                     loglevel++;
    // //             //                 break;
    // //         },

    // if let Some(&d) = matches.get_one::<u8>("diagnostics") {
    //     LOGLEVEL.store(d, Ordering::SeqCst);
    // }
    // //         V_UPPER => {
    // //             //                 print_version();
    // //             //                 exit(EXIT_SUCCESS);
    // //         },
    // //         _ => {
    // //             // usage(stderr);
    // //             // exit(EXIT_FAILURE);
    // //         },
    // //     }
    // // }
    Ok(())
}
