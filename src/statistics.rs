use tracing::event;
use tracing::Level;

use crate::client::Client;
use crate::time::epochms;

pub(crate) struct Statistics {
    pub(crate) connects: u64,
    pub(crate) milliseconds: u128,
    pub(crate) bytes_sent: usize,
}

impl Statistics {
    pub(crate) fn new() -> Self {
        Self {
            bytes_sent: 0,
            connects: 0,
            milliseconds: 0,
        }
    }

    pub(crate) fn log_totals(&self, clients: &[Client]) {
        let mut milliseconds = self.milliseconds;

        let now = epochms();

        for client in clients {
            milliseconds += now - client.connect_time;
        }

        event!(
            Level::INFO,
            "TOTALS connects={} seconds={}.{:03} bytes={}",
            self.connects,
            milliseconds / 1000,
            milliseconds % 1000,
            self.bytes_sent,
        );
    }
}
