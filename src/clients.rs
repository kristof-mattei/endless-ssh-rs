use std::collections::VecDeque;

use time::{Duration, OffsetDateTime};

use crate::client::Client;
use crate::config::Config;
use crate::sender;

pub(crate) struct QueueProcessingResult {
    pub(crate) wait_until: Option<Duration>,
    pub(crate) bytes_sent: usize,
    pub(crate) time_spent: Duration,
}

pub(crate) struct Clients {
    clients: VecDeque<Client>,
}

impl std::ops::Deref for Clients {
    type Target = VecDeque<Client>;

    fn deref(&self) -> &Self::Target {
        &self.clients
    }
}

impl std::ops::DerefMut for Clients {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.clients
    }
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self {
            clients: VecDeque::new(),
        }
    }

    pub(crate) fn destroy_clients(&mut self) -> Duration {
        let mut time_spent = Duration::ZERO;

        for c in self.clients.drain(..) {
            time_spent += c.destroy();
        }

        time_spent
    }

    pub(crate) fn process_queue(&mut self, config: &Config) -> QueueProcessingResult {
        let now = OffsetDateTime::now_utc();

        let mut milliseconds = Duration::ZERO;
        let mut bytes_sent = 0;
        let mut timeout = None;

        // iterate over the queue
        while let Some(potential_client) = self.clients.front() {
            if potential_client.send_next <= now {
                // client is a valid candidate to get a line sent
                let mut client = self
                    .clients
                    .pop_front()
                    .expect("pop_front() after front() failed, universe is broken");

                match sender::sendline(&mut client.tcp_stream, config.max_line_length.get()) {
                    Ok(result) => {
                        // Sometimes things happen that aren't fatal
                        // in which case we couldn't send any results
                        if let Some(sent) = result {
                            bytes_sent += sent;
                            client.bytes_sent += sent;
                        }

                        // in either case, we're re-scheduling this client for later
                        client.send_next = now + config.delay;

                        // and put it in the back
                        self.clients.push_back(client);
                    },
                    Err(_) => {
                        // fatal error sending data to the client
                        milliseconds += client.destroy();
                    },
                }
            } else {
                // no more clients which are processable
                // the timeout is this client (i.e. the next one coming)
                timeout = Some(potential_client.send_next - now);

                break;
            }
        }

        // no more clients
        // no timeout until someone connects
        QueueProcessingResult {
            wait_until: timeout,
            bytes_sent,
            time_spent: milliseconds,
        }
    }
}
