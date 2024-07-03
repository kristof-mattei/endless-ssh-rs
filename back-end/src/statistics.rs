use time::Duration;
use tracing::{event, Level};

use crate::client::Client;

pub(crate) struct Statistics {
    pub(crate) bytes_sent: usize,
    pub(crate) connects: u64,
    pub(crate) lost_clients: u64,
    pub(crate) processed_clients: u64,
    pub(crate) time_spent: Duration,
}

impl Default for Statistics {
    fn default() -> Self {
        Statistics::new()
    }
}

impl Statistics {
    pub(crate) fn new() -> Self {
        Self {
            bytes_sent: 0,
            connects: 0,
            lost_clients: 0,
            processed_clients: 0,
            time_spent: Duration::ZERO,
        }
    }

    pub(crate) fn log_totals<'c, S: 'c>(&self, clients: impl IntoIterator<Item = &'c Client<S>>) {
        let mut time_spent = self.time_spent;
        let mut bytes_sent = self.bytes_sent;

        for client in clients {
            time_spent += client.time_spent;
            bytes_sent += client.bytes_sent;
        }

        event!(
            Level::INFO,
            connects = self.connects,
            time_spent = format_args!(
                "{} week(s), {} day(s), {} hour(s), {} minute(s), {}.{:03} second(s)",
                time_spent.whole_weeks(),
                time_spent.whole_days(),
                time_spent.whole_hours(),
                time_spent.whole_minutes(),
                time_spent.whole_seconds(),
                time_spent.subsec_milliseconds()
            ),
            ?bytes_sent,
            "TOTALS",
        );
    }
}
