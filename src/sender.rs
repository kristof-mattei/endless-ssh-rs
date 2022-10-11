use std::{
    io::{ErrorKind, Write},
    net::TcpStream,
};

use tracing::{event, Level};

use crate::line::randline;

pub(crate) fn sendline(
    tcp_stream: &mut TcpStream,
    max_line_length: usize,
) -> Result<Option<usize>, std::io::Error> {
    let bytes = randline(max_line_length);

    match tcp_stream.write_all(bytes.as_slice()) {
        Ok(()) => {
            let bytes_sent = bytes.len();

            event!(Level::DEBUG, ?bytes_sent);

            Ok(Some(bytes_sent))
        },
        Err(e) if e.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(Level::DEBUG, ?e);

            Ok(None)
        },
        Err(e) => {
            // in reality something when wrong sending the data. It happens.
            event!(Level::WARN, ?e);

            Err(e)
        },
    }
}
