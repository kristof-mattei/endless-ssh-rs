use std::time::Duration;

use tracing::event;
use tracing::Level;

use crate::client::Client;
use crate::time::duration_since_epoch;

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

        let now = duration_since_epoch();

        for client in clients {
            milliseconds += now - client.connect_time;
        }

        event!(
            Level::INFO,
            connects = self.connects,
            time_spent = format_args!(
                "{}.{:03}",
                self.milliseconds.as_secs(),
                self.milliseconds.subsec_millis()
            ),
            bytes_sent = self.bytes_sent,
            "TOTALS"
        );
    }
}
