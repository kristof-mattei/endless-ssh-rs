use std::io::ErrorKind;
use std::net::{Shutdown, SocketAddr, TcpStream};

use time::{Duration, OffsetDateTime};
use tracing::{event, Level};

use crate::ffi_wrapper::set_receive_buffer_size;

pub(crate) struct Client {
    pub(crate) time_spent: Duration,
    pub(crate) send_next: OffsetDateTime,
    pub(crate) bytes_sent: usize,
    pub(crate) addr: SocketAddr,
    pub(crate) tcp_stream: TcpStream,
}

impl std::fmt::Debug for Client {
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

impl Client {
    pub(crate) fn initialize(
        stream: TcpStream,
        addr: SocketAddr,
        start_sending_at: OffsetDateTime,
    ) -> Option<Self> {
        const SIZE_IN_BYTES: usize = 1;

        let c = Client {
            time_spent: Duration::ZERO,
            send_next: start_sending_at,
            addr,
            bytes_sent: 0,
            tcp_stream: stream,
        };

        // Set the smallest possible recieve buffer. This reduces local
        // resource usage and slows down the remote end.
        match set_receive_buffer_size(&c.tcp_stream, SIZE_IN_BYTES) {
            Err(e) => {
                event!(
                    Level::ERROR,
                    message = "Failed to set the tcp stream's receive buffer",
                    ?e
                );

                None
            },
            Ok(()) => Some(c),
        }
    }
}

impl Drop for Client {
    /// Destroys self returning time spent annoying this client
    fn drop(&mut self) {
        event!(Level::INFO, message = "Dropping client...", addr = %self.addr, time_spent = %self.time_spent, bytes_sent = self.bytes_sent);

        if let Some(e) = self
            .tcp_stream
            .shutdown(Shutdown::Both)
            .err()
            .filter(|e| ErrorKind::NotConnected != e.kind())
        {
            // if we had an error sending data then the shutdown will not work
            // because we're already disconnected
            event!(
                Level::DEBUG,
                message = "Error shutting down connection to client, client still discarded",
                ?e
            );
        }
    }
}
