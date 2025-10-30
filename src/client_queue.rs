use std::num::NonZeroUsize;

use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{Level, event};

use crate::client::Client;
use crate::sender;
use crate::statistics::StatisticsMessage;

pub async fn process_clients(
    cancellation_token: CancellationToken,
    delay: std::time::Duration,
    max_line_length: NonZeroUsize,
    client_sender: UnboundedSender<Client<TcpStream>>,
    mut client_receiver: UnboundedReceiver<Client<TcpStream>>,
    statistics_sender: UnboundedSender<StatisticsMessage>,
) {
    let _guard = cancellation_token.clone().drop_guard();

    event!(Level::INFO, "Processing clients");

    loop {
        tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                break;
            },
            received_client = client_receiver.recv() => {
                let Some(client) = received_client else {
                    event!(Level::ERROR, "Client receiver gone");

                    break;
                };

                let Some(client) = process_client(client, cancellation_token.clone(), delay, max_line_length, &statistics_sender).await else {
                    event!(Level::INFO, "Client gone");

                    // no client to re-schedule
                    continue;
                };


                let Ok(()) = client_sender.send(client) else {
                    event!(Level::ERROR, "Client sender gone");

                    break;
                };
            },
        }
    }
}

async fn process_client<S>(
    mut client: Client<S>,
    cancellation_token: CancellationToken,
    delay: std::time::Duration,
    max_line_length: NonZeroUsize,
    statistics_sender: &UnboundedSender<StatisticsMessage>,
) -> Option<Client<S>>
where
    S: tokio::io::AsyncWriteExt + std::marker::Unpin + std::fmt::Debug,
{
    let now = OffsetDateTime::now_utc();

    let client_send_next = client.send_next();

    if client_send_next > now {
        let until_ready = (client_send_next - now)
            .try_into()
            .expect("`send_next` is larger than `now`, so duration should be positive");

        event!(Level::TRACE, addr = ?client.addr(), ?until_ready, "Scheduled client");

        tokio::select! {
            biased;
            () = cancellation_token.cancelled() => {
                // abandon
                return None;
            },
            () = sleep(until_ready) => {}
        }
    }

    statistics_sender
        .send(StatisticsMessage::ProcessedClient)
        .expect("Channel should always exist");

    event!(Level::DEBUG, addr = ?client.addr(), "Processing client");

    if let Ok(bytes_sent) =
        sender::sendline(&mut client.tcp_stream_mut(), max_line_length.into()).await
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

        // and delay again
        *client.send_next_mut() = OffsetDateTime::now_utc() + delay;

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
