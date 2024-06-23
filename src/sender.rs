use std::io::{ErrorKind, Write};

use tracing::{event, Level};

use crate::client::Client;
use crate::config::Config;
use crate::line::randline;

pub(crate) fn sendline(client: &mut Client, config: &Config) -> Result<usize, ()> {
    let bytes = randline(config.max_line_length.get());

    match client.tcp_stream.write_all(bytes.as_slice()) {
        Ok(()) => {
            event!(Level::INFO, message = "Data sent", ?client.addr, bytes_sent = ?bytes.len());

            Ok(bytes.len())
        },
        Err(e) if e.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(Level::DEBUG, message = "Couldn't send anything to client, will try later", ?client.addr, ?e);

            Ok(0)
        },
        Err(error) => {
            // in reality something when wrong sending the data. It happens.
            match error.kind() {
                ErrorKind::ConnectionReset | ErrorKind::TimedOut | ErrorKind::BrokenPipe => {
                    event!(Level::INFO, message = "Failed to send data to client, client gone", ?client.addr, ?error);
                },
                _ => {
                    event!(Level::WARN, message = "Failed to send data to client", ?client.addr, ?error);
                },
            }

            Err(())
        },
    }
}
