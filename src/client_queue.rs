use std::sync::Arc;

use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{RwLock, Semaphore};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{Level, event};

use crate::client::Client;
use crate::config::Config;
use crate::sender;
use crate::statistics::Statistics;

pub async fn process_clients_forever(
    config: Arc<Config>,
    token: CancellationToken,
    client_sender: Sender<Client<TcpStream>>,
    mut client_receiver: Receiver<Client<TcpStream>>,
    semaphore: Arc<Semaphore>,
    statistics: Arc<RwLock<Statistics>>,
) {
    let _guard = token.clone().drop_guard();

    event!(Level::INFO, message = "Processing clients");

    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => {
                break;
            },
            received_client = client_receiver.recv() => {
                if let Some(client) = received_client {
                    if let Some(client) = process_client(client, &semaphore, &config, &statistics).await
                        && (client_sender.send(client).await).is_err() {
                            event!(Level::ERROR, "Client sender gone");
                            break;
                        }
                } else {
                    event!(Level::ERROR, "Client receiver gone");
                    break;
                }
            },
        }
    }
}

async fn process_client<S>(
    mut client: Client<S>,
    semaphore: &Semaphore,
    config: &Config,
    statistics: &RwLock<Statistics>,
) -> Option<Client<S>>
where
    S: tokio::io::AsyncWriteExt + std::marker::Unpin + std::fmt::Debug,
{
    let now = OffsetDateTime::now_utc();

    if client.send_next > now {
        let until_ready = (client.send_next - now)
            .try_into()
            .expect("send_next is larger than now, so duration should be positive");

        event!(Level::TRACE, message = "Scheduled client", addr=?client.addr, ?until_ready);

        sleep(until_ready).await;
    }

    {
        let mut guard = statistics.write().await;
        guard.processed_clients += 1;
    }

    event!(Level::DEBUG, message = "Processing client", addr=?client.addr);

    if let Ok(bytes_sent) =
        sender::sendline(&mut client.tcp_stream, config.max_line_length.into()).await
    {
        client.bytes_sent += bytes_sent;
        client.time_spent += config.delay;

        {
            let mut guard = statistics.write().await;
            guard.bytes_sent += bytes_sent;
            guard.time_spent += config.delay;
        }

        // and delay again
        client.send_next = OffsetDateTime::now_utc() + config.delay;

        // Done processing, return
        Some(client)
    } else {
        {
            let mut guard = statistics.write().await;
            guard.lost_clients += 1;
        }

        // client gone, add back 1 permit
        semaphore.add_permits(1);

        event!(Level::TRACE, message = "Client gone", ?client);

        // can't process, don't return. Client will be dropped, connections terminated by libc::close
        None
    }
}
