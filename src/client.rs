use crate::line::randline;
use crate::log::LogLevel;
use crate::logmsg;
use crate::time::epochms;

use std::ffi::CStr;
use std::io::Error;
use std::mem::MaybeUninit;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::os::unix::prelude::AsRawFd;
use std::ptr::addr_of;

use libc::__errno_location;
use libc::c_int;
use libc::c_void;
use libc::setsockopt;
use libc::strerror;
use libc::write;
use libc::EAGAIN;
use libc::EINTR;
use libc::EWOULDBLOCK;
use libc::SOL_SOCKET;
use libc::SO_RCVBUF;

pub(crate) struct Client {
    pub(crate) ipaddr: IpAddr,
    pub(crate) connect_time: u128,
    pub(crate) send_next: u128,
    pub(crate) bytes_sent: u64,
    pub(crate) port: u16,
    pub(crate) fd: TcpStream,
}

impl Client {
    pub(crate) fn new(fd: TcpStream, addr: SocketAddr, send_next: u128) -> Self {
        let c = Client {
            ipaddr: addr.ip(),
            connect_time: epochms(),
            send_next,
            bytes_sent: 0,
            fd,
            port: addr.port(),
        };
        //         // Set the smallest possible recieve buffer. This reduces local
        //          * resource usage and slows down the remote end.
        //
        let value: i32 = 1;

        #[allow(clippy::cast_possible_truncation)]
        let r: c_int = unsafe {
            setsockopt(
                c.fd.as_raw_fd(),
                SOL_SOCKET,
                SO_RCVBUF,
                addr_of!(value).cast::<c_void>(),
                std::mem::size_of_val(&value) as u32,
            )
        };
        logmsg(
            LogLevel::Debug,
            format!(
                "setsockopt({}, SO_RCVBUF, {}) = {}",
                c.fd.as_raw_fd(),
                value,
                r
            ),
        );

        if r == -1 {
            let error = Error::last_os_error();

            logmsg(LogLevel::Debug, format!("err {}", error));
        }

        c
    }

    pub(crate) fn destroy(self) -> u128 {
        logmsg(LogLevel::Debug, format!("close({})", self.fd.as_raw_fd()));
        let dt = epochms() - self.connect_time;

        logmsg(
            LogLevel::Info,
            format!(
                "CLOSE host={} port={} fd={} time={}.{:03} bytes={}",
                self.ipaddr,
                self.port,
                self.fd.as_raw_fd(),
                dt / 1000,
                dt % 1000,
                self.bytes_sent
            ),
        );

        // STATISTICS.milliseconds += dt;
        drop(self.fd);

        dt
    }
    // Write a line to a client, returning client if it's still up.
    pub(crate) fn sendline(mut self, max_line_length: usize) -> Option<(Self, Option<u64>)> {
        let mut line = unsafe { MaybeUninit::<[MaybeUninit<u8>; 256]>::uninit().assume_init() };
        let len = randline(&mut line, max_line_length);
        loop {
            let out = unsafe { write(self.fd.as_raw_fd(), line.as_ptr().cast::<c_void>(), len) };

            logmsg(
                LogLevel::Debug,
                format!("write({}) = {}", self.fd.as_raw_fd(), out),
            );

            if out == -1 {
                let errno = unsafe { *__errno_location() };
                let msg = unsafe { strerror(errno) };

                logmsg(
                    LogLevel::Debug,
                    format!(
                        "errno = {}, {}",
                        errno,
                        unsafe { CStr::from_ptr(msg) }.to_string_lossy()
                    ),
                );

                match errno {
                    EINTR => {
                        // try again
                        continue;
                    },
                    #[allow(unreachable_patterns)]
                    // EAGAIN == EWOULDBLOCK, but we're converting, not making choices
                    EAGAIN | EWOULDBLOCK => {
                        // don't care
                        return Some((self, None));
                    },
                    _ => {
                        self.destroy();
                        return None;
                    },
                }
            }

            let bytes_sent = u64::try_from(out).expect("Sent negative bytes");

            self.bytes_sent += bytes_sent;

            return Some((self, Some(bytes_sent)));
        }
    }
}
