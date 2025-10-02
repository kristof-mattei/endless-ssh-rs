use time::Duration;
use tokio::sync::mpsc::{self, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{Level, event};

use crate::signal_handlers;

type StdDuration = std::time::Duration;

pub enum StatisticsMessage {
    ProcessedClient,
    LostClient,
    BytesSent(usize),
    TimeSpent(StdDuration),
    /// Connects += 1
    NewClient,
    LogTotals,
}

pub struct Statistics {
    pub bytes_sent: usize,
    pub connects: u64,
    pub lost_clients: u64,
    pub processed_clients: u64,
    pub time_spent: Duration,
}

impl Statistics {
    pub fn new(
        cancellation_token: CancellationToken,
    ) -> (UnboundedSender<StatisticsMessage>, JoinHandle<Statistics>) {
        let (sender, mut receiver) = mpsc::unbounded_channel::<StatisticsMessage>();

        let task = tokio::task::spawn(async move {
            let mut s = Self {
                bytes_sent: 0,
                connects: 0,
                lost_clients: 0,
                processed_clients: 0,
                time_spent: Duration::ZERO,
            };

            loop {
                tokio::select! {
                    () = cancellation_token.cancelled() => {
                        break;
                    },
                    message = receiver.recv() => {
                        match message {
                            Some(StatisticsMessage::ProcessedClient) => s.processed_clients += 1,
                            Some(StatisticsMessage::LostClient) => s.lost_clients += 1,
                            Some(StatisticsMessage::BytesSent(bytes_sent)) => s.bytes_sent += bytes_sent,
                            Some(StatisticsMessage::TimeSpent(duration)) => s.time_spent += duration,
                            Some(StatisticsMessage::NewClient) => s.connects += 1,
                            Some(StatisticsMessage::LogTotals) => s.log_totals(),
                            None => {
                                // the end
                                break;
                            },
                        }
                    }
                }
            }

            s
        });

        (sender, task)
    }

    pub fn log_totals(&self) {
        let time_spent = self.time_spent;
        let bytes_sent = self.bytes_sent;

        event!(
            Level::INFO,
            connects = self.connects,
            time_spent = format_args!(
                "{} week(s), {} day(s), {} hour(s), {} minute(s), {}.{:03} second(s)",
                time_spent.whole_weeks(),
                time_spent.whole_days(),
                time_spent.whole_hours(),
                time_spent.whole_minutes(),
                time_spent.whole_seconds(),
                time_spent.subsec_milliseconds()
            ),
            ?bytes_sent,
            "TOTALS",
        );
    }
}

pub async fn statistics_sigusr1_handler(
    cancellation_token: CancellationToken,
    statistics_sender: UnboundedSender<StatisticsMessage>,
) {
    let _guard = cancellation_token.clone().drop_guard();

    loop {
        tokio::select! {
            () = cancellation_token.cancelled() => {
                break;
            },
            result = signal_handlers::wait_for_sigusr1() => {
                if let Err(error) = result {
                    event!(
                        Level::ERROR,
                        ?error,
                        "Failed to set up `sigusr1` handler"
                    );

                    break;
                }

                if statistics_sender
                    .send(StatisticsMessage::LogTotals)
                    .is_err()
                {
                    break;
                }
            }
        }
    }

    event!(
        Level::INFO,
        "Statistics channel gone, `sigusr1` handler stopped"
    );
}
