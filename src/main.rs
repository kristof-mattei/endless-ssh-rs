#![allow(clippy::items_after_statements)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::similar_names)]

mod cli;
mod client;
mod config;
mod log;
mod server;
mod time;
// endless-ssh-rs: an SSH tarpit

// #define endless-ssh-rs_VERSION           1.1

use crate::time::epochms;
use cli::parse_cli_params;
use client::Client;
use config::Config;
use config::DEFAULT_CONFIG_FILE;
use log::logmsg;
use server::server_create;

use core::slice;
use libc::__errno_location;
use libc::accept;
use libc::c_void;
use libc::close;
use libc::exit;
use libc::fcntl;
use libc::poll;
use libc::pollfd;
use libc::sigaction;
use libc::signal;
use libc::sigset_t;
use libc::strerror;
use libc::write;
use libc::EAGAIN;
use libc::ECONNABORTED;
use libc::EINTR;
use libc::EMFILE;
use libc::ENFILE;
use libc::ENOBUFS;
use libc::ENOMEM;
use libc::EPROTO;
use libc::EWOULDBLOCK;
use libc::EXIT_FAILURE;
use libc::F_GETFL;
use libc::F_SETFL;
use libc::O_NONBLOCK;
use libc::POLLIN;
use libc::SIGHUP;
use libc::SIGPIPE;
use libc::SIGTERM;
use libc::SIGUSR1;
use libc::SIG_IGN;
use log::LogLevel;
use rand::thread_rng;
use rand::Rng;
use std::collections::VecDeque;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::ptr::addr_of_mut;
use std::ptr::null_mut;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

#[derive(Default)]
struct Statistics {
    connects: u64,
    milliseconds: u128,
    bytes_sent: u64,
}

impl Statistics {
    fn statistics_log_totals(&self, clients: &VecDeque<Client>) {
        let mut milliseconds = self.milliseconds;

        let now = epochms();
        for client in clients {
            milliseconds += now - client.connect_time;
        }
        logmsg(
            LogLevel::Info,
            format!(
                "TOTALS connects={} seconds={}.{:03} bytes={}",
                unsafe { STATISTICS.connects },
                milliseconds / 1000,
                milliseconds % 1000,
                unsafe { STATISTICS.bytes_sent },
            ),
        );
    }
}

static mut STATISTICS: Statistics = Statistics {
    bytes_sent: 0,
    milliseconds: 0,
    connects: 0,
};

fn destroy_clients(clients: &mut VecDeque<Client>) {
    for mut c in clients.drain(..) {
        unsafe {
            STATISTICS.milliseconds += c.client_destroy();
        }
    }
}

fn die() {
    let errno = unsafe { *__errno_location() };
    let msg = unsafe { strerror(errno) };
    eprintln!(
        "endless-ssh-rs: fatal: {}",
        unsafe { CStr::from_ptr(msg) }.to_string_lossy()
    );
    unsafe {
        exit(EXIT_FAILURE);
    }
}

fn randline(line: &mut [MaybeUninit<u8>], maxlen: usize) -> usize {
    let len = thread_rng().gen_range(3..=(maxlen - 2));

    for l in line.iter_mut().take(len - 2) {
        let v = thread_rng().gen_range(32..=(32 + 95));
        l.write(v);
    }

    line[len - 2].write(13);
    line[len - 1].write(10);

    let first_4 = unsafe {
        // MaybeUninit::slice_assume_init_ref(&line[..4])
        slice::from_raw_parts(addr_of!(line).cast::<u8>(), 4)
    };

    if *first_4 == [b'S', b'S', b'H', b'-'] {
        line[0].write(b'X');
    }

    len
}

static RUNNING: AtomicBool = AtomicBool::new(true);

#[no_mangle]
pub extern "C" fn sigterm_handler(_signal: u32) {
    RUNNING.store(false, Ordering::SeqCst);
}

static RELOAD: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn sighup_handler(_signal: u32) {
    RELOAD.store(true, Ordering::SeqCst);
}

static DUMPSTATS: AtomicBool = AtomicBool::new(false);

#[no_mangle]
pub extern "C" fn sigusr1_handler(_signal: u32) {
    DUMPSTATS.store(true, Ordering::SeqCst);
}

// enum ConfigKey {
//     KEY_INVALID,
//     KEY_PORT,
//     KEY_DELAY,
//     KEY_MAX_LINE_LENGTH,
//     KEY_MAX_CLIENTS,
//     KEY_LOG_LEVEL,
//     KEY_BIND_FAMILY,
// }

// impl From<&CStr> for ConfigKey {
//     fn from(_: &CStr) -> Self {
//         //     static const char *const table[] = {
//         //         [KEY_PORT]            = "Port",
//         //         [KEY_DELAY]           = "Delay",
//         //         [KEY_MAX_LINE_LENGTH] = "MaxLineLength",
//         //         [KEY_MAX_CLIENTS]     = "MaxClients",
//         //         [KEY_LOG_LEVEL]       = "LogLevel",
//         //         [KEY_BIND_FAMILY]     = "BindFamily"
//         //     };
//         //     for (size_t i = 1; i < sizeof(table) / sizeof(*table); i++)
//         //         if (!strcmp(tok, table[i]))
//         //             return i;
//         //     return KEY_INVALID;
//         todo!()
//     }
// }

// print_version(void)
// {
//     puts("endless-ssh-rs " XSTR(endless-ssh-rs_VERSION));
// }

/* Write a line to a client, returning client if it's still up. */
fn sendline(mut client: Client, max_line_length: usize) -> Option<Client> {
    let mut line = unsafe { MaybeUninit::<[MaybeUninit<u8>; 256]>::uninit().assume_init() };
    let len = randline(&mut line, max_line_length);
    loop {
        let out = unsafe { write(client.fd, line.as_ptr().cast::<c_void>(), len) };
        logmsg(LogLevel::Debug, format!("write({}) = {}", client.fd, out));
        if out == -1 {
            let errno = unsafe { *__errno_location() };

            match errno {
                EINTR => {
                    // try again
                    continue;
                },
                #[allow(unreachable_patterns)]
                // EAGAIN == EWOULDBLOCK, but we're converting, not making choices
                EAGAIN | EWOULDBLOCK => {
                    // don't care
                    return Some(client);
                },
                _ => {
                    client.client_destroy();
                    return None;
                },
            }
        }

        client.bytes_sent += out as u64;
        unsafe { STATISTICS.bytes_sent += out as u64 };
        return Some(client);
    }
}

fn main() {
    let mut config = Config::default();
    let mut config_file = DEFAULT_CONFIG_FILE;

    // #if defined(__OpenBSD__)
    //     unveil(config_file, "r"); /* return ignored as the file may not exist */
    //     if (pledge("inet stdio rpath unveil", 0) == -1)
    //         die();
    // #endif

    config.config_load(config_file);

    parse_cli_params(&mut config);

    //     if (argv[optind]) {
    //         fprintf(stderr, "endless-ssh-rs: too many arguments\n");
    //         exit(EXIT_FAILURE);
    //     }

    //     if (logmsg == logsyslog) {
    //         /* Prepare the syslog */
    //         const char *prog = strrchr(argv[0], '/');
    //         prog = prog ? prog + 1 : argv[0];
    //         openlog(prog, LOG_PID, LOG_DAEMON);
    //     } else {
    //         /* Set output (log) to line buffered */
    //         setvbuf(stdout, 0, _IOLBF, 0);
    //     }

    // Log configuration
    config.log();

    /* Install the signal handlers */
    unsafe {
        signal(SIGPIPE, SIG_IGN);
    }
    {
        let sa = sigaction {
            sa_sigaction: sigterm_handler as usize,
            sa_flags: 0,
            sa_mask: unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() },
            sa_restorer: None,
        };
        let r = unsafe { sigaction(SIGTERM, &sa, null_mut()) };
        if r == -1 {
            die();
        }
    }
    {
        let sa = sigaction {
            sa_sigaction: sighup_handler as usize,
            sa_flags: 0,
            sa_mask: unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() },
            sa_restorer: None,
        };
        let r = unsafe { sigaction(SIGHUP, &sa, null_mut()) };
        if r == -1 {
            die();
        }
    }
    {
        let sa = sigaction {
            sa_sigaction: sigusr1_handler as usize,
            sa_flags: 0,
            sa_mask: unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() },
            sa_restorer: None,
        };
        let r = unsafe { sigaction(SIGUSR1, &sa, null_mut()) };
        if r == -1 {
            die();
        }
    }

    let mut clients = VecDeque::new();

    let mut server = server_create(config.port.into(), config.bind_family);

    while RUNNING.load(Ordering::SeqCst) {
        if RELOAD.load(Ordering::SeqCst) {
            /* Configuration reload requested (SIGHUP) */
            let oldport = config.port;
            let oldfamily = config.bind_family;
            config.config_load(config_file);
            config.log();

            if oldport != config.port || oldfamily != config.bind_family {
                unsafe {
                    close(server);
                }
                server = server_create(config.port.into(), config.bind_family);
            }
            RELOAD.store(false, Ordering::SeqCst);
        }

        if DUMPSTATS.load(Ordering::SeqCst) {
            /* print stats requested (SIGUSR1) */
            unsafe {
                STATISTICS.statistics_log_totals(&clients);
            }
            DUMPSTATS.store(false, Ordering::SeqCst);
        }

        /* Enqueue clients that are due for another message */
        let mut timeout: i32 = -1;
        let now = epochms();
        while let Some(c) = clients.front() {
            if c.send_next <= now {
                let c = clients.pop_front().unwrap();
                if let Some(mut c) = sendline(c, config.max_line_length.get()) {
                    c.send_next = now + u128::from(config.delay.get());
                    clients.push_back(c);
                }
            } else {
                timeout = (c.send_next - now) as i32;
                break;
            }
        }

        // Wait for next event
        let mut fds: pollfd = pollfd {
            fd: server,
            events: POLLIN,
            revents: 0,
        };

        let nfds = (clients.len() < config.max_clients.get())
            .then_some(1)
            .unwrap_or(0);

        logmsg(LogLevel::Debug, format!("poll({}, {})", nfds, timeout));

        let r = unsafe { poll(addr_of_mut!(fds), nfds, timeout) };
        logmsg(LogLevel::Debug, format!("= {}", r));

        if r == -1 {
            let errno = unsafe { *__errno_location() };
            match errno {
                EINTR => {
                    logmsg(LogLevel::Debug, "EINTR");
                    continue;
                },
                _ => {
                    let msg = unsafe { strerror(errno) };
                    eprintln!(
                        "endless-ssh-rs: fatal: {}",
                        unsafe { CStr::from_ptr(msg) }.to_string_lossy()
                    );
                    unsafe {
                        exit(EXIT_FAILURE);
                    }
                },
            }
        }

        /* Check for new incoming connections */
        if fds.revents & POLLIN == POLLIN {
            let fd = unsafe { accept(server, null_mut(), null_mut()) };
            logmsg(LogLevel::Debug, format!("accept() = {}", fd));
            unsafe { STATISTICS.connects += 1 };
            if fd == -1 {
                let errno = unsafe { *__errno_location() };
                let msg = unsafe { strerror(errno) };

                match errno {
                    EMFILE | ENFILE => {
                        // config.max_clients = clients.len();
                        // logmsg(LogLevel::Info, format!("MaxClients {}", clients.len()));
                        logmsg(
                            LogLevel::Info,
                            format!(
                                "Unable to accept new connection due to {}",
                                unsafe { CStr::from_ptr(msg) }.to_string_lossy()
                            ),
                        );
                    },
                    ECONNABORTED | EINTR | ENOBUFS | ENOMEM | EPROTO => {
                        eprintln!(
                            "endless-ssh-rs: warning: {}",
                            unsafe { CStr::from_ptr(msg) }.to_string_lossy()
                        );
                    },
                    _ => {
                        eprintln!(
                            "endless-ssh-rs: fatal: {}",
                            unsafe { CStr::from_ptr(msg) }.to_string_lossy()
                        );
                        unsafe { exit(EXIT_FAILURE) };
                    },
                }
            } else {
                let send_next = epochms() + u128::from(config.delay.get());
                let client = Client::new(fd, send_next);
                unsafe {
                    let flags = fcntl(fd, F_GETFL, 0); /* cannot fail */
                    fcntl(fd, F_SETFL, flags | O_NONBLOCK); /* cannot fail */
                }
                let message = format!(
                    "ACCEPT host={} port={} fd={} n={}/{}",
                    String::from_utf8_lossy(&client.ipaddr),
                    client.port,
                    client.fd,
                    clients.len(),
                    config.max_clients
                );
                clients.push_back(client);
                logmsg(LogLevel::Info, message);
            }
        }
    }

    destroy_clients(&mut clients);
    unsafe {
        STATISTICS.statistics_log_totals(&clients);
    }

    //     if (logmsg == logsyslog)
    //         closelog();
}

#[cfg(test)]
mod tests {
    use std::mem::MaybeUninit;

    use crate::randline;

    #[test]
    fn test_randline() {
        let max_line_length = 100;
        let test_cases: [(&str, u64); 10] = [
            ("v\r\n", 13_622_895_711_870),
            (
                "SK<j/-\"W!yE[s4X\"vR$j?fS^B:<;o6}m^Q\\8X~5DsfZ@X<! G#Zy!fAua'FGZDwRy{^Q&$9NgtE)9N1iC\r\n",
                7_751_687_408_060_890_519,
            ),
            (
                "1cru4S>\\BftX2K|Nh.?#Pbs<>o>B;21U0IMf0&]gW><%,QMiA9G\\p}:(9E*S?9$\r\n",
                9_005_229_067_430_428_354,
            ),
            (
                "m*H :xZapSQdAb2kY[Z.a]A):R@JX/D=oA1Bg-&}:#}iRX\r\n",
                3_077_303_310_408_858_050,
            ),
            (
                "=B~*?0vf?N.a^(H7K)g8qYnt'(BQv+I%A9\"o>4N4|h_]&_F.Der@CL\r\n",
                11_124_280_525_600_449_503,
            ),
            (
                "zgT56V%kn;jlm?kT<U#B#U=R#kDXB(\\l!$d=\r\n",
                3_234_824_119_897_000_326,
            ),
            ("W +?<mA =(Gj,V\"r.#0\r\n", 13_853_042_096_933_652_546),
            ("5\r\n", 8_524_234_894_527_582_045),
            ("c^7SFXx*;\r\n", 5_082_429_743_911_758_750),
            (
                "A$l^Lp5/i\\0~g<pxaYrlf5&Ub=>hgy.kCVmP3Yf]Ly6QLAT`PtqJ7oTD'%]bo2CSf/S\"O7uY_?o;_&#3gsP_`Z}\\}\r\n",
                3_816_722_555_100_047_063,
            ),
        ];
        // for (mut rng, expected, expected_rng) in tests {
        for (i, (expected, expected_rng)) in test_cases.into_iter().enumerate() {
            let mut line = unsafe { MaybeUninit::<[MaybeUninit<u8>; 256]>::uninit().assume_init() };
            let mut rng = i as u64;
            let len = randline(&mut line, max_line_length);

            let random_line = {
                // MaybeUninit::slice_assume_init_ref(s)
                unsafe { &*(std::ptr::addr_of!(line[0..len]) as *const [u8]) }
            };

            assert_eq!(String::from_utf8_lossy(random_line), expected);
            assert_eq!(rng, expected_rng);
        }
    }
}
