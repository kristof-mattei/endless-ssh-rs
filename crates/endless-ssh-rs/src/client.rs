use std::net::SocketAddr;

use time::Duration;
use tokio::sync::OwnedSemaphorePermit;
use tracing::{Level, event};

pub struct Client<S> {
    time_spent: Duration,
    bytes_sent: usize,
    addr: SocketAddr,
    tcp_stream: S,
    permit: OwnedSemaphorePermit,
}

impl<S> std::fmt::Debug for Client<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("time_spent", &self.time_spent)
            .field("bytes_sent", &self.bytes_sent)
            .field("addr", &self.addr)
            // .field("tcp_stream", &self.tcp_stream)
            .finish_non_exhaustive()
    }
}

impl<S> Client<S> {
    pub fn new(stream: S, addr: SocketAddr, permit: OwnedSemaphorePermit) -> Self {
        Self {
            time_spent: Duration::ZERO,
            addr,
            bytes_sent: 0,
            tcp_stream: stream,
            permit,
        }
    }

    #[expect(unused, reason = "Consistency with other props")]
    pub fn time_spent(&self) -> Duration {
        self.time_spent
    }

    pub fn time_spent_mut(&mut self) -> &mut Duration {
        &mut self.time_spent
    }

    #[expect(unused, reason = "Consistency with other props")]
    pub fn bytes_sent(&self) -> usize {
        self.bytes_sent
    }

    pub fn bytes_sent_mut(&mut self) -> &mut usize {
        &mut self.bytes_sent
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn tcp_stream_mut(&mut self) -> &mut S {
        &mut self.tcp_stream
    }
}

impl<S> Drop for Client<S> {
    /// Destroys `self` returning time spent annoying this client.
    fn drop(&mut self) {
        event!(
            Level::INFO,
            addr = %self.addr,
            time_spent = %self.time_spent,
            bytes_sent = self.bytes_sent,
            "Dropping client...",
        );

        // no need to shut down the stream, it happens when it is dropped

        // Technically this client's permit isn't available until AFTER this function has ended
        let available_slots = self.permit.semaphore().available_permits() + 1;

        event!(Level::INFO, available_slots);
    }
}
