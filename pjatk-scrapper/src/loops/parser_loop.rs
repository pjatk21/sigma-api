use std::time::Duration;

use chrono::DateTime;
use crossbeam::utils::Backoff;
use futures::{TryFutureExt};
use tokio::{sync::broadcast::{Sender, error::RecvError}};
use tracing::{info_span, info, warn};

use crate::scraper::{EntryToSend, parse_timetable_day};


pub(crate) struct ParserLoop<'a> {
    tx: Sender<EntryToSend>,
    client: &'a reqwest::Client,
}

impl<'a> ParserLoop<'a> {
    pub(crate) fn new(
        tx: Sender<EntryToSend>,
        client: &'a reqwest::Client,
    ) -> Self {
        Self {
            tx,
            client,
        }
    }

    pub(crate) async fn start(&mut self) {
        let mut rx = self.tx.subscribe();
        loop {
            let entry_result = rx.recv().await;
            let backoff = Backoff::new();
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::HypervisorCommand(hypervisor_command) => {
                        let date = DateTime::parse_from_rfc3339(&hypervisor_command.scrapUntil)
                            .expect("Bad DateTime format until!");
                        let date_str = date.format("%Y-%m-%d").to_string();
                        parse_timetable_day(&self.client, date_str, self.tx.clone())
                            .and_then(|_| async {
                                let span = info_span!("Parsing entries");
                                span.in_scope(|| {
                                    info!("Scrapping ended!: {}", date);
                                });

                                self.tx
                                    .send(EntryToSend::HypervisorFinish("finished"))
                                    .expect("`finish`-ing failed!");
                                Ok(())
                            })
                            .await
                            .expect("Parsing failed!");
                    }
                    EntryToSend::Quit => {
                        let span = info_span!("Parsing entries");
                        span.in_scope(|| {
                            info!("Closing scraper thread!");
                        });
                        break;
                    }
                    _ => {
                        backoff.snooze();
                    }
                },
                Err(recv_error) => match recv_error {
                    RecvError::Lagged(number) => {
                        warn!("Receiver overflow! Skipping: {}", number);
                        backoff.snooze();
                    }
                    RecvError::Closed => {
                        break;
                    }
                },
            }
            tokio::time::sleep(Duration::from_nanos(250)).await;
        }
    }
}
