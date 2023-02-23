use std::collections::VecDeque;

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
        if self.is_empty() {
            return QueueProcessingResult::default();
        }

        let now = OffsetDateTime::now_utc();

        let mut disconnected_clients_time_spent = Duration::ZERO;
        let mut disconnected_clients_bytes_sent = 0;
        let mut timeout = None;
        // just for logging
        let mut processed_clients = 0;

        let clients_going_in = self.clients.len();

        event!(
            Level::INFO,
            message = "Processing (part of) queue",
            total_clients = clients_going_in,
        );

        // iterate over the queue
        while let Some(potential_client) = self.clients.front() {
            event!(
                Level::TRACE,
                message = "Considering client",
                ?potential_client,
                ?now
            );

            if potential_client.send_next <= now {
                processed_clients += 1;

                // client is a valid candidate to get a line sent
                let client = self
                    .clients
                    .pop_front()
                    .expect("pop_front() after front() failed, universe is broken");

                event!(Level::DEBUG, message = "Processing", ?client);

                match sender::sendline(client, config) {
                    Ok((mut client, bytes_sent)) => {
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

                        // and put it in the back
                        self.clients.push_back(client);
                    },
                    Err((client_time_spent, client_bytes_sent)) => {
                        disconnected_clients_time_spent += client_time_spent;
                        disconnected_clients_bytes_sent += client_bytes_sent;
                    },
                }
            } else {
                // no more clients which are processable
                // the timeout is this client (i.e. the next one coming)
                timeout = Some(potential_client.send_next - now);

                event!(
                    Level::TRACE,
                    message = "No (more) clients eligible.",
                    ?potential_client,
                    ?timeout,
                    ?now
                );
                break;
            }
        }

        if processed_clients == 0 {
            event!(
                Level::WARN,
                message = "Processed no clients. If we just had a new client this is expected"
            );
        } else {
            let total_clients = self.clients.len();

            event!(
                Level::INFO,
                message = "Processed (part of) queue",
                processed_clients,
                lost_clients = clients_going_in - total_clients,
                total_clients = total_clients,
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
