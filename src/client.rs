use crate::ffi_wrapper::set_receive_buffer_size;
use crate::line::randline;
use crate::time::milliseconds_since_epoch;

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

#[derive(Debug)]
pub(crate) struct Client {
    pub(crate) ipaddr: IpAddr,
    pub(crate) connect_time: u128,
    pub(crate) send_next: u128,
    pub(crate) bytes_sent: usize,
    pub(crate) port: u16,
    pub(crate) tcp_stream: TcpStream,
}

impl Client {
    pub(crate) fn new(fd: TcpStream, addr: SocketAddr, send_next: u128) -> Self {
        const SIZE_IN_BYTES: usize = 1;

        let c = Client {
            ipaddr: addr.ip(),
            connect_time: milliseconds_since_epoch(),
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
    pub(crate) fn destroy(self) -> u128 {
        let dt = milliseconds_since_epoch() - self.connect_time;

        event!(
            Level::INFO,
            "CLOSE host={} port={} stream={:?} time={}.{:03} bytes={}",
            self.ipaddr,
            self.port,
            self.tcp_stream,
            dt / 1000,
            dt % 1000,
            self.bytes_sent
        );

        event!(Level::DEBUG, "Shutting down {:?}", self);

        if let Err(e) = self.tcp_stream.shutdown(Shutdown::Both) {
            // warn because we're destroying.
            event!(Level::WARN, ?e);
        }

        dt
    }

    /// Write a line to a client. Consumes the client. If the client is still up, return the client.
    #[instrument]
    pub(crate) fn sendline(&mut self, max_line_length: usize) -> Result<Option<usize>, Error> {
        let buffer = randline(max_line_length);

        match self.tcp_stream.write_all(buffer.as_slice()) {
            Ok(()) => {
                let bytes_sent = buffer.len();
                self.bytes_sent += bytes_sent;

                // event!(Level::DEBUG, ?self, bytes_sent);

                Ok(Some(bytes_sent))
            },
            Err(e) if e.kind() == ErrorKind::WouldBlock => {
                // EAGAIN, EWOULDBLOCK

                event!(Level::DEBUG, ?self, ?e);

                Ok(None)
            },
            Err(e) => {
                event!(Level::ERROR, ?self, ?e);

                // TODO log
                Err(e)
            },
        }
    }
}
