use std::sync::atomic::Ordering;

use libc::{SIGINT, SIGTERM, SIGUSR1};
use tracing::{event, Level};

use crate::ffi_wrapper::set_up_handler;
use crate::{DUMPSTATS, RUNNING};

#[no_mangle]
extern "C" fn sigterm_handler(_signal: i32) {
    event!(Level::INFO, "Stopping the engine");
    RUNNING.store(false, Ordering::SeqCst);
}

#[no_mangle]
extern "C" fn sigusr1_handler(_signal: i32) {
    DUMPSTATS.store(true, Ordering::SeqCst);
}

// Set up the signal handlers
pub(crate) fn setup_handlers() -> Result<(), color_eyre::Report> {
    // SIGPIPE is ignored by default in Rust
    // set_up_handler(SIGPIPE, SIG_IGN)?;
    set_up_handler(SIGTERM, sigterm_handler)?;
    set_up_handler(SIGINT, sigterm_handler)?;
    set_up_handler(SIGUSR1, sigusr1_handler)?;

    Ok(())
}
