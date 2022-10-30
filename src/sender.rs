use std::io::ErrorKind;
use std::io::Write;

use tracing::{event, Level};

use crate::{client::Client, config::Config, line::randline};

pub(crate) fn sender(
    mut client: Client,
    config: &Config,
) -> Result<(Client, usize), (time::Duration, usize)> {
    let bytes = randline(config.max_line_length.get());

    match client.tcp_stream.write_all(bytes.as_slice()) {
        Ok(()) => {
            event!(Level::DEBUG, message = "Successfully sent bytes to client", ?client.addr, bytes_sent = ?bytes.len());

            Ok((client, bytes.len()))
        },
        Err(e) if e.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(Level::DEBUG, message = "Couldn't send anything to client, will try later", ?client.addr, ?e);

            Ok((client, 0))
        },
        Err(e) => {
            // in reality something when wrong sending the data. It happens.
            event!(Level::WARN, message = "Failed to send data to client", ?client.addr, ?e);

            Err((client.time_spent, client.bytes_sent))
        },
    }
}
