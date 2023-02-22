use std::fmt::Display;

use libc::timespec;
use time::Duration;

pub(crate) enum Timeout {
    Infinite,
    Duration(Duration),
}

impl std::fmt::Debug for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Infinite => write!(f, "Infinite"),
            Self::Duration(arg0) => write!(f, "{}", arg0),
        }
    }
}

impl Timeout {
    pub(crate) fn as_c_timeout(&self) -> i32 {
        // note the + 1
        // Duration stores data as seconds and nanoseconds internally.
        // if the nanoseconds < 1 milliseconds it gets lost
        // so we add one to make sure we always wait until the duration has passed
        match self {
            Timeout::Infinite => -1,
            Timeout::Duration(m) => i32::try_from(m.whole_milliseconds() + 1).unwrap_or(i32::MAX),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_c_timespec(&self) -> Option<timespec> {
        match self {
            Timeout::Infinite => None,
            Timeout::Duration(m) => Some(timespec {
                tv_sec: m.whole_seconds(),
                tv_nsec: m.subsec_nanoseconds().into(),
            }),
        }
    }
}

impl Display for Timeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.as_c_timeout()))
    }
}

impl From<Option<Duration>> for Timeout {
    fn from(duration: Option<Duration>) -> Self {
        match duration {
            None => Timeout::Infinite,
            Some(d) => Timeout::Duration(d),
        }
    }
}
