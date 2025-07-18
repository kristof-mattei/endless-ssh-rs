use std::io::Error;
use std::ptr::null_mut;

use color_eyre::eyre;
use libc::{c_int, sigaction};
use tracing::Level;

use crate::wrap_and_report;

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
