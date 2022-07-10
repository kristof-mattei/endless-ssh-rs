use crate::client::Client;
use crate::log::logmsg;
use crate::log::LogLevel;
use crate::time::epochms;

#[derive(Default)]
pub(crate) struct Statistics {
    pub(crate) connects: u64,
    pub(crate) milliseconds: u128,
    pub(crate) bytes_sent: u64,
}

impl Statistics {
    pub(crate) fn log_totals(&self, clients: &[Client]) {
        let mut milliseconds = self.milliseconds;

        let now = epochms();
        for client in clients {
            milliseconds += now - client.connect_time;
        }

        logmsg(
            LogLevel::Info,
            format!(
                "TOTALS connects={} seconds={}.{:03} bytes={}",
                self.connects,
                milliseconds / 1000,
                milliseconds % 1000,
                self.bytes_sent,
            ),
        );
    }
}
