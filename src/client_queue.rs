use std::collections::VecDeque;

use time::{Duration, OffsetDateTime};
use tracing::{event, Level};

use crate::client::Client;
use crate::config::Config;
use crate::sender;

pub(crate) struct QueueProcessingResult {
    pub(crate) wait_until: Option<Duration>,
    pub(crate) bytes_sent: usize,
    pub(crate) time_spent: Duration,
}

pub(crate) struct ClientQueue {
    clients: VecDeque<Client>,
}

impl std::ops::Deref for ClientQueue {
    type Target = VecDeque<Client>;

    fn deref(&self) -> &Self::Target {
        &self.clients
    }
}

impl std::ops::DerefMut for ClientQueue {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.clients
    }
}

impl Default for ClientQueue {
    fn default() -> Self {
        ClientQueue::new()
    }
}

impl ClientQueue {
    pub(crate) fn new() -> Self {
        Self {
            clients: VecDeque::new(),
        }
    }

    pub(crate) fn destroy_clients(&mut self) -> Duration {
        let mut time_spent = Duration::ZERO;

        for c in self.clients.drain(..) {
            time_spent += c.time_spent;

            // c goes out of scope and gets dropped
        }

        time_spent
    }

    pub(crate) fn process_queue(&mut self, config: &Config) -> QueueProcessingResult {
        let now = OffsetDateTime::now_utc();

        let mut milliseconds = Duration::ZERO;
        let mut bytes_sent = 0;
        let mut timeout = None;

        event!(Level::INFO, message = "Processing clients");

        // iterate over the queue
        while let Some(potential_client) = self.clients.front() {
            if potential_client.send_next <= now {
                // client is a valid candidate to get a line sent
                let client = self
                    .clients
                    .pop_front()
                    .expect("pop_front() after front() failed, universe is broken");

                event!(Level::DEBUG, message = "Sending data to", ?client.addr);

                match sender::sendline(client, config) {
                    Ok((mut client, bytes_sent)) => {
                        client.bytes_sent += bytes_sent;
                        client.time_spent += config.delay;
                        client.send_next = now + config.delay;

                        // and put it in the back
                        self.clients.push_back(client);
                    },
                    Err((d, b)) => {
                        milliseconds += d;
                        bytes_sent += b;
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
