use crate::die;
use crate::log::LogLevel;

use libc::__errno_location;
use libc::bind;
use libc::c_void;
use libc::in_addr;
use libc::listen;
use libc::setsockopt;
use libc::sockaddr;
use libc::sockaddr_in;
use libc::sockaddr_in6;
use libc::socket;
use libc::socklen_t;
use libc::strerror;
use libc::AF_INET;
use libc::AF_INET6;
use libc::AF_UNSPEC;
use libc::INADDR_ANY;
use libc::INT_MAX;
use libc::SOCK_STREAM;
use libc::SOL_SOCKET;
use libc::SO_REUSEADDR;
use std::ffi::CStr;
use std::ptr::addr_of;

use crate::log::logmsg;
pub(crate) fn server_create(port: u16, family: i32) -> i32 {
    let s = unsafe {
        socket(
            if family == AF_UNSPEC {
                AF_INET6
            } else {
                family
            },
            SOCK_STREAM,
            0,
        )
    };

    logmsg(LogLevel::Debug, format!("socket() = {}", s));

    if s == -1 {
        die();
    }

    // Socket options are best effort, allowed to fail
    let value = 1;

    let mut r = unsafe {
        setsockopt(
            s,
            SOL_SOCKET,
            SO_REUSEADDR,
            addr_of!(value).cast::<c_void>(),
            socklen_t::try_from(std::mem::size_of_val(&value)).expect("Value too large"),
        )
    };
    logmsg(
        LogLevel::Debug,
        format!("setsockopt({}, SO_REUSEADDR, true) = {}", s, r),
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

    /*
     * With OpenBSD IPv6 sockets are always IPv6-only, so the socket option
     * is read-only (not modifiable).
     * http://man.openbsd.org/ip6#IPV6_V6ONLY
     */
    // #ifndef __OpenBSD__
    //     if (family == AF_INET6 || family == AF_UNSPEC) {
    //         errno = 0;
    //         value = (family == AF_INET6);
    //         r = setsockopt(s, IPPROTO_IPV6, IPV6_V6ONLY, &value, sizeof(value));
    //         logmsg(log_debug, "setsockopt(%d, IPV6_V6ONLY, true) = %d", s, r);
    //         if (r == -1)
    //             logmsg(log_debug, "errno = %d, %s", errno, strerror(errno));
    //     }
    // #endif

    if family == AF_INET {
        let addr4 = sockaddr_in {
            sin_family: AF_INET as u16,
            sin_port: port.to_be(),
            sin_addr: in_addr { s_addr: INADDR_ANY },
            sin_zero: [0; 8],
        };

        r = unsafe {
            bind(
                s,
                addr_of!(addr4).cast::<sockaddr>(),
                std::mem::size_of_val(&addr4) as u32,
            )
        };
    } else {
        let addr6 = sockaddr_in6 {
            sin6_family: AF_INET6 as u16,
            sin6_port: port.to_be(),
            sin6_addr: libc::in6_addr {
                s6_addr: [0; 16], /* in6addr_any */
            },
            sin6_flowinfo: 0,
            sin6_scope_id: 0,
        };
        r = unsafe {
            bind(
                s,
                addr_of!(addr6).cast::<sockaddr>(),
                std::mem::size_of::<sockaddr_in6>() as socklen_t,
            )
        };
    }
    logmsg(
        LogLevel::Debug,
        format!("bind({}, port={}) = {}", s, port, r),
    );
    if r == -1 {
        die();
    }

    r = unsafe { listen(s, INT_MAX) };

    logmsg(LogLevel::Debug, format!("listen({}) = {}", s, r));
    if r == -1 {
        die();
    }

    s
}
