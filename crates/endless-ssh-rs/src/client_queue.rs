use std::num::NonZeroU8;

use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio_stream::StreamExt as _;
use tokio_util::sync::CancellationToken;
use tokio_util::time::DelayQueue;
use tracing::{Level, event};

use crate::client::Client;
use crate::sender;
use crate::statistics::StatisticsMessage;

pub async fn process_clients(
    cancellation_token: CancellationToken,
    delay: std::time::Duration,
    max_line_length: NonZeroU8,
    mut client_receiver: UnboundedReceiver<Client<TcpStream>>,
    statistics_sender: UnboundedSender<StatisticsMessage>,
) {
    let _guard = cancellation_token.clone().drop_guard();

    event!(Level::INFO, "Processing clients");

    let mut clients = DelayQueue::<Client<TcpStream>>::new();

    loop {
        tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                break;
            },
            Some(expired) = clients.next() => {
                let Some(client) = process_client(expired.into_inner(), delay, max_line_length, &statistics_sender).await else {
                    event!(Level::INFO, "Client gone");

                    // no client to re-schedule
                    continue;
                };

                clients.insert(client, delay);
            },
            received_client = client_receiver.recv() => {
                let Some(client) = received_client else {
                    event!(Level::ERROR, "Client receiver gone");

                    break;
                };

                event!(Level::TRACE, addr = ?client.addr(), "Scheduled client");

                clients.insert(client, delay);
            },
        }
    }
}

async fn process_client<S>(
    mut client: Client<S>,
    delay: std::time::Duration,
    max_line_length: NonZeroU8,
    statistics_sender: &UnboundedSender<StatisticsMessage>,
) -> Option<Client<S>>
where
    S: tokio::io::AsyncWriteExt + std::marker::Unpin + std::fmt::Debug,
{
    statistics_sender
        .send(StatisticsMessage::ProcessedClient)
        .expect("Channel should always exist");

    event!(Level::DEBUG, addr = ?client.addr(), "Processing client");

    if let Ok(bytes_sent) =
        sender::sendline(&mut client.tcp_stream_mut(), max_line_length.get().into()).await
    {
        *client.bytes_sent_mut() += bytes_sent;
        *client.time_spent_mut() += delay;

        {
            statistics_sender
                .send(StatisticsMessage::BytesSent(bytes_sent))
                .expect("Channel should always exist");
            statistics_sender
                .send(StatisticsMessage::TimeSpent(delay))
                .expect("Channel should always exist");
        }

        // Done processing, return
        Some(client)
    } else {
        {
            statistics_sender
                .send(StatisticsMessage::LostClient)
                .expect("Channel should always exist");
        }

        event!(Level::TRACE, ?client, "Client gone");

        // can't process, don't return to queue.
        // Client will be dropped, connections terminated by libc::close
        // and permit will be returned
        None
    }
}
