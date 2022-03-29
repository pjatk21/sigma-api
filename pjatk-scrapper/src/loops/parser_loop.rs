use std::{time::Duration, sync::Arc};

use chrono::DateTime;
use crossbeam::utils::Backoff;
use futures::{TryFutureExt};
use thirtyfour::{WebDriver, By, Keys};
use tokio::{sync::broadcast::{Sender, error::RecvError}};
use tracing::{info_span, info, warn};

use crate::scraper::{EntryToSend, parse_timetable_day};


pub(crate) struct ParserLoop {
    tx: Sender<EntryToSend>,
    client: Arc<WebDriver>,
}

impl ParserLoop {
    pub(crate) fn new(
        tx: Sender<EntryToSend>,
        client: Arc<WebDriver>,
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
                                self.client
                                    .refresh()
                                    .await
                                    .expect("Refreshing page failed!");
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

                        let window = self
                            .client
                            .find_element(By::Css("html"))
                            .await
                            .expect("Find element failed!");
                        window
                            .send_keys(Keys::Alt + Keys::F4)
                            .await
                            .expect("Close window failed! Stop geckodriver container manually!");
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
