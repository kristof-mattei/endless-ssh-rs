use crate::ffi_wrapper::set_receive_buffer_size;
use crate::line::randline;
use crate::time::duration_since_epoch;
use crate::time::format_duration;

use tracing::event;
use tracing::instrument;
use tracing::Level;

use std::io::Error;
use std::io::ErrorKind;
use std::io::Write;
use std::net::IpAddr;
use std::net::Shutdown;
use std::net::SocketAddr;
use std::net::TcpStream;
use std::time::Duration;

pub(crate) struct Client {
    pub(crate) ipaddr: IpAddr,
    pub(crate) connect_time: Duration,
    pub(crate) send_next: Duration,
    pub(crate) bytes_sent: usize,
    pub(crate) port: u16,
    pub(crate) tcp_stream: TcpStream,
}

impl std::fmt::Debug for Client {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client")
            .field("ipaddr", &self.ipaddr)
            .field("connect_time", &format_duration(&self.connect_time))
            .field("send_next", &format_duration(&self.send_next))
            .field("bytes_sent", &self.bytes_sent)
            .field("port", &self.port)
            .field("tcp_stream", &self.tcp_stream)
            .finish()
    }
}

impl Client {
    pub(crate) fn new(fd: TcpStream, addr: SocketAddr, send_next: Duration) -> Self {
        const SIZE_IN_BYTES: usize = 1;

        let c = Client {
            ipaddr: addr.ip(),
            connect_time: duration_since_epoch(),
            send_next,
            bytes_sent: 0,
            tcp_stream: fd,
            port: addr.port(),
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

    // Consumes the client. Shuts down the TCP connection.
    #[instrument]
    pub(crate) fn destroy(self) -> Duration {
        let time_spent = duration_since_epoch() - self.connect_time;

        event!(Level::INFO, ?time_spent);

        if let Err(e) = self.tcp_stream.shutdown(Shutdown::Both) {
            // if we had an error sending data then the shutdown will not work
            // because we're already disconnected
            if ErrorKind::NotConnected != e.kind() {
                // warn because we're destroying.
                event!(Level::WARN, ?e);
            }
        }

        time_spent
    }

    /// Write a line to a client. Consumes the client. If the client is still up, return the client.
    #[instrument]
    pub(crate) fn sendline(&mut self, max_line_length: usize) -> Result<Option<usize>, Error> {
        let buffer = randline(max_line_length);

        match self.tcp_stream.write_all(buffer.as_slice()) {
            Ok(()) => {
                let bytes_sent = buffer.len();

                event!(Level::DEBUG, ?bytes_sent);

                self.bytes_sent += bytes_sent;

                Ok(Some(bytes_sent))
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // EAGAIN, EWOULDBLOCK

                event!(Level::DEBUG, ?e);

                Ok(None)
            },
            Err(e) if e.kind() == ErrorKind::BrokenPipe => {
                event!(Level::DEBUG, ?e);

                Err(e)
            },
            Err(e) => {
                event!(Level::ERROR, ?e);

                Err(e)
            },
        }
    }
}
