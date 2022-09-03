use std::collections::VecDeque;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::client::Client;
use crate::config::Config;
use crate::time::milliseconds_since_epoch;

pub(crate) struct QueueProcessingResult {
    pub(crate) timeout: Option<u128>,
    pub(crate) bytes_sent: usize,
    pub(crate) milliseconds: u128,
}

pub(crate) struct Clients(VecDeque<Client>);

impl Deref for Clients {
    type Target = VecDeque<Client>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Clients {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self(VecDeque::<Client>::new())
    }

    pub(crate) fn destroy_clients(&mut self) -> u128 {
        let mut time_spent = 0;

        for c in self.drain(..) {
            time_spent += c.destroy();
        }

        time_spent
    }

    pub(crate) fn process_queue(&mut self, config: &Config) -> QueueProcessingResult {
        let now = milliseconds_since_epoch();

        let mut milliseconds = 0;
        let mut bytes_sent = 0;
        let mut timeout = None;

        // iterate over the queue
        while let Some(potential_client) = self.front() {
            if potential_client.send_next <= now {
                // client is a valid candidate to get a line sent
                let mut client = self
                    .pop_front()
                    .expect("pop_front() after front() failed, universe is broken");

                match client.sendline(config.max_line_length.get()) {
                    Ok(result) => {
                        // Sometimes things happen that aren't fatal
                        // in which case we couldn't send any results
                        if let Some(sent) = result {
                            bytes_sent += sent;
                        }

                        // in either case, we're re-scheduling this client for later
                        client.send_next = now + u128::from(config.delay_ms.get());

                        // and put it in the back
                        self.push_back(client);
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
            timeout,
            bytes_sent,
            milliseconds,
        }
    }
}
