use std::io::ErrorKind;

use tracing::{Level, event};

use crate::line::randline;

pub(crate) async fn sendline(
    target: &mut (impl tokio::io::AsyncWriteExt + std::marker::Unpin + std::fmt::Debug),
    max_length: usize,
) -> Result<usize, ()> {
    let bytes = randline(max_length);

    match target.write_all(bytes.as_slice()).await {
        Ok(()) => {
            event!(
                Level::TRACE,
                ?target,
                bytes_sent = ?bytes.len(),
                "Data sent",
            );

            Ok(bytes.len())
        },
        Err(error) if error.kind() == ErrorKind::WouldBlock => {
            // EAGAIN, EWOULDBLOCK
            event!(
                Level::DEBUG,
                ?target,
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
                        ?target,
                        ?error,
                        "Failed to send data to client, client gone",
                    );
                },
                _ => {
                    event!(
                        Level::WARN,
                        ?target,
                        ?error,
                        "Failed to send data to client"
                    );
                },
            }

            Err(())
        },
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use crate::sender::sendline;

    #[derive(Debug)]
    struct ErrorWrite {
        error: ErrorKind,
    }

    impl tokio::io::AsyncWrite for ErrorWrite {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            _buf: &[u8],
        ) -> std::task::Poll<Result<usize, std::io::Error>> {
            std::task::Poll::Ready(Err(std::io::Error::from(self.error)))
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            unreachable!()
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), std::io::Error>> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn test_ok() {
        #[derive(Debug)]
        struct OkWrite {
            written: usize,
        }

        impl tokio::io::AsyncWrite for OkWrite {
            fn poll_write(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
                buf: &[u8],
            ) -> std::task::Poll<Result<usize, std::io::Error>> {
                self.get_mut().written = buf.len();
                std::task::Poll::Ready(Ok(buf.len()))
            }

            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), std::io::Error>> {
                unreachable!()
            }

            fn poll_shutdown(
                self: std::pin::Pin<&mut Self>,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), std::io::Error>> {
                unreachable!()
            }
        }

        let ok_write = OkWrite { written: 0 };

        tokio::pin!(ok_write);

        let r = sendline(&mut ok_write, 100).await;

        assert_eq!(Ok(ok_write.written), r);
    }

    #[tokio::test]
    async fn test_fail_not_connected() {
        let error_not_connected = ErrorWrite {
            error: ErrorKind::NotConnected,
        };

        tokio::pin!(error_not_connected);

        let r = sendline(&mut error_not_connected, 100).await;

        assert_eq!(Err(()), r);
    }

    #[tokio::test]
    async fn test_pass_would_block() {
        let error_would_block = ErrorWrite {
            error: ErrorKind::WouldBlock,
        };

        tokio::pin!(error_would_block);

        let r = sendline(&mut error_would_block, 100).await;

        assert_eq!(Ok(0), r);
    }

    #[tokio::test]
    async fn test_error_connection_reset() {
        let error_connection_reset = ErrorWrite {
            error: ErrorKind::ConnectionReset,
        };

        tokio::pin!(error_connection_reset);

        let r = sendline(&mut error_connection_reset, 100).await;

        assert_eq!(Err(()), r);
    }
}
