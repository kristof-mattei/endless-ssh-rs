use std::num::NonZeroU16;
use std::num::NonZeroU32;
use std::num::NonZeroUsize;

use libc::c_int;
use libc::AF_INET;
use libc::AF_INET6;
use libc::AF_UNSPEC;

use crate::log::logmsg;
use crate::log::LogLevel;

pub(crate) const DEFAULT_PORT: u16 = 2223; // 1 -> 65535

// milliseconds
pub(crate) const DEFAULT_DELAY: u32 = 10000;
pub(crate) const DEFAULT_MAX_LINE_LENGTH: u64 = 32;
pub(crate) const DEFAULT_MAX_CLIENTS: u64 = 4096;

// #if defined(__FreeBSD__)
// #  define DEFAULT_CONFIG_FILE "/usr/local/etc/endless-ssh-rs.config"
// #else
pub(crate) const DEFAULT_CONFIG_FILE: &str = "/etc/endless-ssh-rs/config";
// #endif

const DEFAULT_BIND_FAMILY: c_int = AF_UNSPEC;

pub(crate) struct Config {
    pub(crate) port: NonZeroU16,
    pub(crate) delay: NonZeroU32,
    pub(crate) max_line_length: NonZeroUsize,
    pub(crate) max_clients: NonZeroUsize,
    pub(crate) bind_family: i32, // TODO
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
            bind_family: DEFAULT_BIND_FAMILY,
        }
    }
}

impl Config {
    pub(crate) fn set_port(&mut self, port: NonZeroU16) -> Result<(), ()> {
        self.port = port;

        Ok(())
    }

    pub(crate) fn set_delay(&mut self, delay: NonZeroU32) -> Result<(), ()> {
        self.delay = delay;
        Ok(())
    }

    pub(crate) fn set_max_clients(&mut self, max_clients: NonZeroUsize) -> Result<(), ()> {
        self.max_clients = max_clients;
        Ok(())
    }

    fn set_max_line_length(&mut self, l: NonZeroUsize) -> Result<(), ()> {
        if l.get() < 3 || l.get() > 255 {
            eprintln!("endless-ssh-rs: Invalid line length: {}", l.get());
            Err(())
        } else {
            self.max_line_length = l;
            Ok(())
        }
    }

    fn set_bind_family_ipv4(&mut self) -> Result<(), ()> {
        self.bind_family = AF_INET;
        Ok(())
    }

    fn set_bind_family_ipv6(&mut self) -> Result<(), ()> {
        self.bind_family = AF_INET6;
        Ok(())
    }

    fn set_bind_family_unspecified(&mut self) -> Result<(), ()> {
        self.bind_family = AF_UNSPEC;
        Ok(())
    }

    pub(crate) fn config_load(&mut self, s: &str) {

        //     long lineno = 0;
        //     FILE *f = fopen(file, "r");
        //     if (f) {
        //         char line[256];
        //         while (fgets(line, sizeof(line), f)) {
        //             lineno++;

        //             /* Remove comments */
        //             char *comment = strchr(line, '#');
        //             if (comment)
        //                 *comment = 0;

        //             /* Parse tokes on line */
        //             char *save = 0;
        //             char *tokens[3];
        //             int ntokens = 0;
        //             for (; ntokens < 3; ntokens++) {
        //                 char *tok = strtok_r(ntokens ? 0 : line, " \r\n", &save);
        //                 if (!tok)
        //                     break;
        //                 tokens[ntokens] = tok;
        //             }

        //             switch (ntokens) {
        //                 case 0: /* Empty line */
        //                     continue;
        //                 case 1:
        //                     fprintf(stderr, "%s:%ld: Missing value\n", file, lineno);
        //                     if (hardfail) exit(EXIT_FAILURE);
        //                     continue;
        //                 case 2: /* Expected */
        //                     break;
        //                 case 3:
        //                     fprintf(stderr, "%s:%ld: Too many values\n", file, lineno);
        //                     if (hardfail) exit(EXIT_FAILURE);
        //                     continue;
        //             }

        //             enum config_key key = config_key_parse(tokens[0]);
        //             switch (key) {
        //                 case KEY_INVALID:
        //                     fprintf(stderr, "%s:%ld: Unknown option '%s'\n",
        //                             file, lineno, tokens[0]);
        //                     break;
        //                 case KEY_PORT:
        //                     config_set_port(c, tokens[1], hardfail);
        //                     break;
        //                 case KEY_DELAY:
        //                     config_set_delay(c, tokens[1], hardfail);
        //                     break;
        //                 case KEY_MAX_LINE_LENGTH:
        //                     config_set_max_line_length(c, tokens[1], hardfail);
        //                     break;
        //                 case KEY_MAX_CLIENTS:
        //                     config_set_max_clients(c, tokens[1], hardfail);
        //                     break;
        //                 case KEY_BIND_FAMILY:
        //                     config_set_bind_family(c, tokens[1], hardfail);
        //                     break;
        //                 case KEY_LOG_LEVEL: {
        //                     errno = 0;
        //                     char *end;
        //                     long v = strtol(tokens[1], &end, 10);
        //                     if (errno || *end || v < log_none || v > log_debug) {
        //                         fprintf(stderr, "%s:%ld: Invalid log level '%s'\n",
        //                                 file, lineno, tokens[1]);
        //                         if (hardfail) exit(EXIT_FAILURE);
        //                     } else {
        //                         loglevel = v;
        //                     }
        //                 } break;
        //             }
        //         }

        //         fclose(f);
        //     }
    }

    pub(crate) fn log(&self) {
        logmsg(LogLevel::Info, format!("Port {}", self.port));
        logmsg(LogLevel::Info, format!("Delay {}", self.delay));
        logmsg(
            LogLevel::Info,
            format!("MaxLineLength {}", self.max_line_length),
        );
        logmsg(LogLevel::Info, format!("MaxClients {}", self.max_clients));
        let bind_family_description = match self.bind_family {
            AF_INET6 => "Ipv6 Only",
            AF_INET => "Ipv4 Only",
            _ => "IPv4 Mapped IPv6",
        };
        logmsg(
            LogLevel::Info,
            format!("BindFamily {}", bind_family_description),
        );
    }
}
