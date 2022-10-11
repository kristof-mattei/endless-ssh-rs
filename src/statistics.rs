use time::{Duration, OffsetDateTime};

use tracing::event;
use tracing::Level;

use crate::client::Client;

pub(crate) struct Statistics {
    pub(crate) connects: u64,
    pub(crate) milliseconds: Duration,
    pub(crate) bytes_sent: usize,
}

impl Statistics {
    pub(crate) fn new() -> Self {
        Self {
            bytes_sent: 0,
            connects: 0,
            milliseconds: Duration::ZERO,
        }
    }

    pub(crate) fn log_totals<'c>(&self, clients: impl IntoIterator<Item = &'c Client>) {
        let mut milliseconds = self.milliseconds;

        let now = OffsetDateTime::now_utc();

        for client in clients {
            milliseconds += client.connect_time - now;
        }

        event!(
            Level::INFO,
            connects = self.connects,
            time_spent = format_args!(
                "{}.{:03}",
                self.milliseconds.whole_seconds(),
                self.milliseconds.subsec_milliseconds()
            ),
            bytes_sent = self.bytes_sent,
            "TOTALS"
        );
    }
}
