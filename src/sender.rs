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
            event!(
                Level::INFO,
                ?addr,
                bytes_sent = ?bytes.len(),
                "Data sent",
            );

            Ok(bytes.len())
        },
        Err(error) if error.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(
                Level::DEBUG,
                ?addr,
                ?error,
                "Couldn't send anything to client, will try later",
            );

            Ok(0)
        },
        Err(error) => {
            // something went wrong sending the data. It happens.
            match error.kind() {
                ErrorKind::ConnectionReset | ErrorKind::TimedOut | ErrorKind::BrokenPipe => {
                    event!(
                        Level::INFO,
                        ?addr,
                        ?error,
                        "Failed to send data to client, client gone",
                    );
                },
                _ => {
                    event!(Level::WARN, ?addr, ?error, "Failed to send data to client");
                },
            }

            Err(())
        },
    }
}

#[cfg(test)]
mod tests {
    use std::{io::ErrorKind, net::IpAddr};

    use crate::sender::sendline;

    struct ErrorWrite {
        error: ErrorKind,
    }

    impl std::io::Write for ErrorWrite {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::from(self.error))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            unreachable!()
        }
    }

    #[test]
    fn test_ok() {
        struct OkWrite {
            written: usize,
        }

        impl std::io::Write for OkWrite {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.written = buf.len();
                Ok(buf.len())
            }

            fn flush(&mut self) -> std::io::Result<()> {
                unreachable!()
            }
        }

        let mut ok_write = OkWrite { written: 0 };

        let r = sendline(
            &mut ok_write,
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            100,
        );

        assert_eq!(Ok(ok_write.written), r);
    }

    #[test]
    fn test_fail_not_connected() {
        let mut error_not_connected = ErrorWrite {
            error: ErrorKind::NotConnected,
        };

        let r = sendline(
            &mut error_not_connected,
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            100,
        );

        assert_eq!(Err(()), r);
    }

    #[test]
    fn test_pass_would_block() {
        let mut error_would_block = ErrorWrite {
            error: ErrorKind::WouldBlock,
        };

        let r = sendline(
            &mut error_would_block,
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            100,
        );

        assert_eq!(Ok(0), r);
    }

    #[test]
    fn test_error_connection_reset() {
        let mut error_connection_reset = ErrorWrite {
            error: ErrorKind::ConnectionReset,
        };
        let r = sendline(
            &mut error_connection_reset,
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            100,
        );

        assert_eq!(Err(()), r);
    }
}
