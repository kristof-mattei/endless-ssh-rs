use std::net::SocketAddr;

use time::{Duration, OffsetDateTime};
use tracing::{Level, event};

pub(crate) struct Client<S> {
    pub(crate) time_spent: Duration,
    pub(crate) send_next: OffsetDateTime,
    pub(crate) bytes_sent: usize,
    pub(crate) addr: SocketAddr,
    pub(crate) tcp_stream: S,
}

impl<S> std::cmp::Eq for Client<S> {}

impl<S> std::cmp::PartialEq for Client<S> {
    fn eq(&self, other: &Self) -> bool {
        self.addr == other.addr
    }
}

impl<S> std::cmp::Ord for Client<S> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // flipped to get the oldest first
        other.send_next.cmp(&self.send_next)
    }
}

impl<S> std::cmp::PartialOrd for Client<S> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<S> std::fmt::Debug for Client<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("time_spent", &self.time_spent)
            .field("send_next", &self.send_next)
            .field("bytes_sent", &self.bytes_sent)
            .field("addr", &self.addr)
            // .field("tcp_stream", &self.tcp_stream)
            .finish_non_exhaustive()
    }
}

impl<S> Client<S> {
    pub(crate) fn new(stream: S, addr: SocketAddr, start_sending_at: OffsetDateTime) -> Self {
        Self {
            time_spent: Duration::ZERO,
            send_next: start_sending_at,
            addr,
            bytes_sent: 0,
            tcp_stream: stream,
        }
    }
}

impl<S> Drop for Client<S> {
    /// Destroys self returning time spent annoying this client
    fn drop(&mut self) {
        event!(
            Level::INFO,
            addr = %self.addr,
            time_spent = %self.time_spent,
            bytes_sent = self.bytes_sent,
            "Dropping client...",
        );

        // no need to shut down the stream, it happens when it is dropped
    }
}
