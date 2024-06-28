use std::collections::binary_heap::PeekMut;
use std::collections::BinaryHeap;

use time::{Duration, OffsetDateTime};
use tracing::{event, Level};

use crate::client::Client;
use crate::config::Config;
use crate::sender;

#[derive(Default)]
pub(crate) struct QueueProcessingResult {
    pub(crate) timeout: Option<Duration>,
    pub(crate) bytes_sent: usize,
    pub(crate) time_spent: Duration,
}

pub(crate) struct ClientQueue<S> {
    clients: BinaryHeap<Client<S>>,
}

impl<S> std::ops::Deref for ClientQueue<S> {
    type Target = BinaryHeap<Client<S>>;

    fn deref(&self) -> &Self::Target {
        &self.clients
    }
}

impl<S> std::ops::DerefMut for ClientQueue<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.clients
    }
}

impl<S> Default for ClientQueue<S> {
    fn default() -> Self {
        ClientQueue::new()
    }
}

impl<S> ClientQueue<S> {
    pub(crate) fn new() -> Self {
        Self {
            clients: BinaryHeap::new(),
        }
    }

    pub(crate) fn destroy_clients(&mut self) -> Duration {
        let mut time_spent = Duration::ZERO;

        for c in self.clients.drain() {
            time_spent += c.time_spent;

            // c goes out of scope and gets dropped
        }

        time_spent
    }

    pub(crate) fn process_queue(&mut self, config: &Config) -> QueueProcessingResult
    where
        S: std::io::Write,
    {
        if self.is_empty() {
            return QueueProcessingResult::default();
        }

        let now = OffsetDateTime::now_utc();

        let mut disconnected_clients_time_spent = Duration::ZERO;
        let mut disconnected_clients_bytes_sent = 0;
        let mut timeout = None;

        // just for logging
        let mut processed_clients: usize = 0;

        let clients_going_in = self.clients.len();

        event!(
            Level::INFO,
            total_clients = clients_going_in,
            "Processing (part of) queue",
        );

        while let Some(mut client) = self.clients.peek_mut() {
            event!(Level::TRACE, ?client, ?now, "Considering client");

            if client.send_next <= now {
                processed_clients += 1;

                event!(Level::DEBUG, ?client, "Processing",);

                let address = client.addr;
                let mut stream = &mut client.tcp_stream;

                if let Ok(bytes_sent) =
                    sender::sendline(&mut stream, address, config.max_line_length.get())
                {
                    client.bytes_sent += bytes_sent;
                    client.time_spent += config.delay;

                    // this will cause all of them to converge
                    // note that we're using a once-set now
                    // and not a per-client to ensure  our loop is finite
                    // if not, we could end up in a situation where processing the loop takes > delay
                    // in which case when we're around we need to restart
                    // and never yield back to the connection processor
                    // we could fix this with an integer trying to determine
                    // how many we processed but that seems cumbersome
                    // as we need to determine then how many we processed MINUS how many failed
                    client.send_next = now + config.delay;
                } else {
                    disconnected_clients_time_spent += client.time_spent;
                    disconnected_clients_bytes_sent += client.bytes_sent;

                    // we got a unrecoverable error, remove them from the equasion
                    PeekMut::pop(client);
                }
            } else {
                // no more clients which are processable
                // the timeout is this client (i.e. the next one coming)
                timeout = Some(client.send_next - now);

                event!(
                    Level::TRACE,
                    ?client,
                    ?timeout,
                    ?now,
                    "No (more) clients eligible.",
                );

                break;
            }
        }

        if processed_clients == 0 {
            event!(
                Level::WARN,
                "Processed no clients. If we just had a new client this is expected",
            );
        } else {
            let total_clients = self.clients.len();

            event!(
                Level::INFO,
                processed_clients,
                lost_clients = clients_going_in - total_clients,
                total_clients = total_clients,
                "Processed (part of) queue",
            );
        }

        // no more clients
        // no timeout until someone connects
        QueueProcessingResult {
            timeout,
            bytes_sent: disconnected_clients_bytes_sent,
            time_spent: disconnected_clients_time_spent,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{sink, ErrorKind};
    use std::net::IpAddr;

    use time::OffsetDateTime;

    use super::ClientQueue;
    use crate::client::Client;
    use crate::config::Config;

    #[test]
    fn test_write() {
        let mut queue = ClientQueue::new();

        queue.push(Client::new(
            sink(),
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            OffsetDateTime::now_utc(),
        ));

        let _r = queue.process_queue(&Config {
            ..Default::default()
        });

        assert_eq!(queue.len(), 1);
        assert!(queue.pop().unwrap().bytes_sent > 1);
    }
    #[test]

    fn test_error_writing() {
        struct NoWrite {}
        impl std::io::Write for NoWrite {
            fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::from(ErrorKind::NotConnected))
            }

            fn flush(&mut self) -> std::io::Result<()> {
                Err(std::io::Error::from(ErrorKind::NotConnected))
            }
        }
        let mut queue = ClientQueue::new();

        queue.push(Client::new(
            NoWrite {},
            std::net::SocketAddr::new(IpAddr::V4([192, 168, 99, 1].into()), 3000),
            OffsetDateTime::now_utc(),
        ));

        let _r = queue.process_queue(&Config {
            ..Default::default()
        });

        assert_eq!(queue.len(), 0);
    }
}
