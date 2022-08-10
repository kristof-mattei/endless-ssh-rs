use std::collections::VecDeque;
use std::ops::Deref;
use std::ops::DerefMut;

use crate::client::Client;
use crate::config::Config;
use crate::time::milliseconds_since_epoch;

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

    pub(crate) fn process_queue(&mut self, config: &Config) -> (i32, usize) {
        let now = milliseconds_since_epoch();

        // TODO this needs to be added to statistics
        #[allow(unused_variables)]
        let mut milliseconds = 0;
        let mut bytes_sent = 0;

        while let Some(c) = self.front() {
            if c.send_next <= now {
                let mut c = self.pop_front().unwrap();

                match c.sendline(config.max_line_length.get()) {
                    Ok(sent) => {
                        if let Some(s) = sent {
                            bytes_sent += s;
                        }
                        c.send_next = now + u128::from(config.delay_ms.get());
                        self.push_back(c);
                    },
                    Err(_) => {
                        milliseconds += c.destroy();
                    },
                }
            } else {
                return (
                    i32::try_from(c.send_next - now).expect("Timeout didn't fit i32"),
                    bytes_sent,
                );
            }
        }

        // TODO this is not a Rust way of returning 'no timeout'
        (-1, bytes_sent)
    }
}
