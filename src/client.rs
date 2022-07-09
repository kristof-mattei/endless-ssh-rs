use crate::log::LogLevel;
use crate::logmsg;
use crate::time::epochms;

use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::addr_of;
use std::ptr::addr_of_mut;
use std::ptr::null_mut;

use libc::__errno_location;
use libc::c_int;
use libc::c_void;
use libc::close;
use libc::getnameinfo;
use libc::getpeername;
use libc::setsockopt;
use libc::sockaddr;
use libc::sockaddr_in;
use libc::sockaddr_in6;
use libc::sockaddr_storage;
use libc::socklen_t;
use libc::strerror;
use libc::AF_INET;
use libc::NI_NUMERICHOST;
use libc::SOL_SOCKET;
use libc::SO_RCVBUF;

// https://pubs.opengroup.org/onlinepubs/9699919799/basedefs/netinet_in.h.html
const INET6_ADDRSTRLEN: usize = 46;

pub(crate) struct Client {
    pub(crate) ipaddr: [u8; INET6_ADDRSTRLEN],
    pub(crate) connect_time: u128,
    pub(crate) send_next: u128,
    pub(crate) bytes_sent: u64,
    pub(crate) port: u16,
    pub(crate) fd: c_int,
}

impl Client {
    pub(crate) fn new(fd: c_int, send_next: u128) -> Self {
        let mut c = Client {
            ipaddr: [0; INET6_ADDRSTRLEN],
            connect_time: epochms(),
            send_next,
            bytes_sent: 0,
            fd,
            port: 0,
        };
        //         /* Set the smallest possible recieve buffer. This reduces local
        //          * resource usage and slows down the remote end.
        //          */
        let value: i32 = 1;

        #[allow(clippy::cast_possible_truncation)]
        let r: c_int = unsafe {
            setsockopt(
                fd,
                SOL_SOCKET,
                SO_RCVBUF,
                addr_of!(value).cast::<c_void>(),
                std::mem::size_of_val(&value) as u32,
            )
        };
        logmsg(
            LogLevel::Debug,
            format!("setsockopt({}, SO_RCVBUF, {}) = {}", fd, value, r),
        );
        if r == -1 {
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
        }

        /* Get IP address */
        let mut addr = MaybeUninit::<sockaddr_storage>::uninit();
        let mut len = std::mem::size_of::<sockaddr_storage>();

        #[allow(clippy::cast_possible_truncation)]
        if unsafe {
            getpeername(
                fd,
                addr_of_mut!(addr).cast::<sockaddr>(),
                addr_of_mut!(len).cast::<socklen_t>(),
            )
        } != -1
        {
            if unsafe { (*addr.as_ptr()).ss_family } == (AF_INET as u16) {
                c.port = unsafe { *addr_of!(addr).cast::<sockaddr_in>() }
                    .sin_port
                    .to_be();

                unsafe {
                    getnameinfo(
                        addr_of!(addr).cast::<sockaddr>(),
                        std::mem::size_of::<sockaddr_in>() as socklen_t,
                        addr_of_mut!(c.ipaddr).cast::<i8>(),
                        INET6_ADDRSTRLEN as socklen_t,
                        null_mut(),
                        0,
                        NI_NUMERICHOST,
                    );
                }
            } else {
                c.port = unsafe { *addr.as_ptr().cast::<sockaddr_in6>() }
                    .sin6_port
                    .to_be();

                unsafe {
                    getnameinfo(
                        addr_of!(addr).cast::<sockaddr>(),
                        std::mem::size_of::<sockaddr_in6>() as socklen_t,
                        addr_of_mut!(c.ipaddr).cast::<i8>(),
                        INET6_ADDRSTRLEN as socklen_t,
                        null_mut(),
                        0,
                        NI_NUMERICHOST,
                    );
                }
            }
        }
        c
    }

    pub(crate) fn client_destroy(&mut self) -> u128 {
        logmsg(LogLevel::Debug, format!("close({})", self.fd));
        let dt = epochms() - self.connect_time;

        logmsg(
            LogLevel::Info,
            format!(
                "CLOSE host={} port={} fd={} time={}.{:03} bytes={}",
                String::from_utf8_lossy(&self.ipaddr),
                self.port,
                self.fd,
                dt / 1000,
                dt % 1000,
                self.bytes_sent
            ),
        );
        unsafe {
            // STATISTICS.milliseconds += dt;
            close(self.fd);
        };

        return dt;
    }
}
