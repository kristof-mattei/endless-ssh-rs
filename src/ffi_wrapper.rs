use std::io::Error;
use std::mem::size_of_val;
use std::os::unix::prelude::AsRawFd as _;
use std::ptr::null_mut;

use color_eyre::eyre;
use libc::{SO_RCVBUF, SOL_SOCKET, c_int, c_void, setsockopt, sigaction, socklen_t};
use tokio::net::TcpStream;
use tracing::Level;

use crate::wrap_and_report;

pub fn set_receive_buffer_size(tcp_stream: &TcpStream, size_in_bytes: usize) -> Result<(), Error> {
    // Set the smallest possible recieve buffer. This reduces local
    // resource usage and slows down the remote end.
    let value: i32 = i32::try_from(size_in_bytes).expect("Byte buffer didn't fit in an i32");

    let size: socklen_t = u32::try_from(size_of_val(&value)).unwrap();

    // SAFETY: external call
    let r: c_int = unsafe {
        setsockopt(
            tcp_stream.as_raw_fd(),
            SOL_SOCKET,
            SO_RCVBUF,
            (&raw const value).cast::<c_void>(),
            size,
        )
    };

    if r == -1 {
        return Err(Error::last_os_error());
    }

    Ok(())
}

#[expect(unused, reason = "Unused")]
pub fn set_up_handler(
    signum: c_int,
    sig_handler: extern "C" fn(_: c_int),
) -> Result<(), eyre::Report> {
    #[cfg(not(target_os = "macos"))]
    // SAFETY: all zeroes are valid for `sigset_t`
    let sa_mask = unsafe { std::mem::MaybeUninit::<libc::sigset_t>::zeroed().assume_init() };

    #[cfg(target_os = "macos")]
    let sa_mask = 0;

    #[expect(
        clippy::as_conversions,
        clippy::fn_to_numeric_cast_any,
        reason = "We actually need the function as a pointer"
    )]
    let sig_handler_ptr = sig_handler as usize;

    let sa = sigaction {
        sa_sigaction: sig_handler_ptr,
        sa_flags: 0,
        sa_mask,
        #[cfg(not(target_os = "macos"))]
        sa_restorer: None,
    };

    // SAFETY: libc call
    if unsafe { sigaction(signum, &raw const sa, null_mut()) } == -1 {
        return Err(wrap_and_report!(
            Level::ERROR,
            Error::last_os_error(),
            "Failure to install signal handler"
        ));
    }

    Ok(())
}
