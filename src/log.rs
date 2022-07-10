use core::sync::atomic::Ordering;
use libc::{__errno_location, syslog};
use std::ffi::CString;
use std::num::NonZeroU8;
use std::sync::atomic::{AtomicBool, AtomicU8};
use time::format_description::well_known::iso8601;
use time::format_description::well_known::iso8601::TimePrecision;
use time::format_description::well_known::Iso8601;

#[derive(Copy, Clone, PartialEq, PartialOrd)]
#[repr(u8)]
pub(crate) enum LogLevel {
    None = 0,
    Info,
    Debug,
}

pub(crate) static LOGLEVEL: AtomicU8 = AtomicU8::new(2);
static LOG_TO_FILE: AtomicBool = AtomicBool::new(false);

pub(crate) fn logmsg(level: LogLevel, message: impl AsRef<str>) {
    let set_log_level: LogLevel = LOGLEVEL
        .load(Ordering::SeqCst)
        .try_into()
        .expect("Unsuppored LogLevel");

    if set_log_level >= level {
        if LOG_TO_FILE.load(Ordering::SeqCst) {
            logsyslog(level, message);
        } else {
            logstdio(message);
        }
    }
}

impl TryFrom<u8> for LogLevel {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            //     value
            0 => Ok(LogLevel::None),
            1 => Ok(LogLevel::Info),
            2 => Ok(LogLevel::Debug),
            v => Err(format!("Couldn't convert {} into `LogLevel`", v)),
        }
    }
}

impl From<LogLevel> for u8 {
    fn from(val: LogLevel) -> Self {
        val as u8
    }
}

fn logstdio(message: impl AsRef<str>) {
    // Print a timestamp
    let now = time::OffsetDateTime::now_utc()
        .format(
            &Iso8601::<
                {
                    iso8601::Config::DEFAULT
                        .set_time_precision(TimePrecision::Second {
                            decimal_digits: NonZeroU8::new(3u8),
                        })
                        .encode()
                },
            >,
        )
        .expect("Unable to format date in Rfc3339 format");

    println!("{} {}", now, message.as_ref());
}

fn logsyslog(level: LogLevel, message: impl AsRef<str>) {
    let save = unsafe { *__errno_location() };

    let r = CString::new(message.as_ref()).expect("Invalid message");

    unsafe {
        syslog(i32::from(u8::from(level)), r.as_ptr());
    }

    unsafe { *__errno_location() = save };
}
