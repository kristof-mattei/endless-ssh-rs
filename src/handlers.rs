use std::io::Error;
use std::mem::MaybeUninit;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;

use libc::c_int;
use libc::sigaction;
use libc::sigset_t;
use libc::SIGINT;
use libc::SIGPIPE;
use libc::SIGTERM;
use libc::SIGUSR1;
use libc::SIG_IGN;
use tracing::event;
use tracing::Level;

use crate::wrap_and_report;
use crate::DUMPSTATS;
use crate::RUNNING;

#[no_mangle]
pub extern "C" fn sigterm_handler(_signal: u32) {
    event!(Level::INFO, "Stopping the engine");
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
        #[cfg(not(target_os = "macos"))]
        sa_restorer: None,
    };

    if unsafe { sigaction(signum, &sa, null_mut()) } == -1 {
        return Err(wrap_and_report!(
            Level::ERROR,
            Error::last_os_error(),
            "Failure to install signal handler"
        ));
    }

    Ok(())
}

pub(crate) fn set_up_handlers() -> Result<(), anyhow::Error> {
    set_up_handler(SIGPIPE, SIG_IGN)?;
    set_up_handler(SIGTERM, sigterm_handler as usize)?;
    set_up_handler(SIGINT, sigterm_handler as usize)?;
    set_up_handler(SIGUSR1, sigusr1_handler as usize)?;

    Ok(())
}
