use crate::DUMPSTATS;
use crate::RUNNING;

use libc::__errno_location;
use libc::c_int;
use libc::sigaction;
use libc::sigset_t;
use libc::strerror;
use libc::SIGPIPE;
use libc::SIGTERM;
use libc::SIGUSR1;
use libc::SIG_IGN;
use std::ffi::CStr;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

#[no_mangle]
pub extern "C" fn sigterm_handler(_signal: u32) {
    RUNNING.store(false, Ordering::SeqCst);
}

#[no_mangle]
pub extern "C" fn sigusr1_handler(_signal: u32) {
    DUMPSTATS.store(true, Ordering::SeqCst);
}

fn set_up_handler(signum: c_int, handler: usize) -> Result<(), anyhow::Error> {
    let sa = sigaction {
        sa_sigaction: handler,
        sa_flags: 0,
        sa_mask: unsafe { MaybeUninit::<sigset_t>::zeroed().assume_init() },
        sa_restorer: None,
    };

    if unsafe { sigaction(signum, &sa, null_mut()) } == -1 {
        let errno = unsafe { *__errno_location() };
        let msg = unsafe { strerror(errno) };

        return Err(anyhow::Error::msg(format!(
            "Failed to install handler, {}",
            unsafe { CStr::from_ptr(msg).to_string_lossy() }
        )));
    }

    Ok(())
}

pub(crate) fn set_up_handlers() -> Result<(), anyhow::Error> {
    set_up_handler(SIGPIPE, SIG_IGN)?;
    set_up_handler(SIGTERM, sigterm_handler as usize)?;
    set_up_handler(SIGUSR1, sigusr1_handler as usize)?;

    Ok(())
}
