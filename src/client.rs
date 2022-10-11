use crate::ffi_wrapper::set_receive_buffer_size;

use time::Duration;
use time::OffsetDateTime;
use tracing::event;
use tracing::instrument;
use tracing::Level;

use std::io::ErrorKind;
use std::net::Shutdown;
use std::net::SocketAddr;
use std::net::TcpStream;

pub(crate) struct Client {
    pub(crate) connect_time: OffsetDateTime,
    pub(crate) send_next: OffsetDateTime,
    pub(crate) bytes_sent: usize,
    pub(crate) addr: SocketAddr,
    pub(crate) tcp_stream: TcpStream,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("connect_time", &self.connect_time)
            .field("send_next", &self.send_next)
            .field("bytes_sent", &self.bytes_sent)
            .field("addr", &self.addr)
            // .field("tcp_stream", &self.tcp_stream)
            .finish()
    }
}

impl Client {
    #[instrument()]
    pub(crate) fn initialize(
        stream: TcpStream,
        addr: SocketAddr,
        start_sending_at: OffsetDateTime,
    ) -> Self {
        const SIZE_IN_BYTES: usize = 1;

        let c = Client {
            connect_time: OffsetDateTime::now_utc(),
            send_next: start_sending_at,
            addr,
            bytes_sent: 0,
            tcp_stream: stream,
        };

        // Set the smallest possible recieve buffer. This reduces local
        // resource usage and slows down the remote end.
        if let Err(e) = set_receive_buffer_size(&c.tcp_stream, SIZE_IN_BYTES) {
            event!(Level::ERROR, ?e);
        } else {
            event!(
                Level::DEBUG,
                "Set the tcp steam's receive buffer to {}",
                SIZE_IN_BYTES
            );
        }

        c
    }

    /// Destroys self returning time spent annoying this client
    #[instrument()]
    pub(crate) fn destroy(self) -> Duration {
        let time_spent = OffsetDateTime::now_utc() - self.connect_time;

        event!(Level::INFO, message = "Disconnecting client...", time_spent = %time_spent);

        if let Err(e) = self.tcp_stream.shutdown(Shutdown::Both) {
            // if we had an error sending data then the shutdown will not work
            // because we're already disconnected
            if ErrorKind::NotConnected != e.kind() {
                event!(Level::DEBUG, ?e);
            }
        }

        time_spent
    }
}
