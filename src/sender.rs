use std::io::ErrorKind;

use tracing::{event, Level};

use crate::line::randline;

pub(crate) fn sendline(
    stream: &mut impl std::io::Write,
    addr: impl std::fmt::Debug,
    max_length: usize,
) -> Result<usize, ()> {
    let bytes = randline(max_length);

    match stream.write_all(bytes.as_slice()) {
        Ok(()) => {
            event!(Level::INFO, message = "Data sent", ?addr, bytes_sent = ?bytes.len());

            Ok(bytes.len())
        },
        Err(e) if e.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(
                Level::DEBUG,
                message = "Couldn't send anything to client, will try later",
                ?addr,
                ?e
            );

            Ok(0)
        },
        Err(error) => {
            // in reality something when wrong sending the data. It happens.
            match error.kind() {
                ErrorKind::ConnectionReset | ErrorKind::TimedOut | ErrorKind::BrokenPipe => {
                    event!(
                        Level::INFO,
                        message = "Failed to send data to client, client gone",
                        ?addr,
                        ?error
                    );
                },
                _ => {
                    event!(
                        Level::WARN,
                        message = "Failed to send data to client",
                        ?addr,
                        ?error
                    );
                },
            }

            Err(())
        },
    }
}
