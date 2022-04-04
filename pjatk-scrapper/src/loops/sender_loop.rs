use std::time::Duration;

use crossbeam::utils::Backoff;
use futures::{stream::SplitSink, SinkExt};
use tokio::{
    net::TcpStream,
    sync::broadcast::{error::RecvError, Receiver},
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{error, error_span, info, info_span, warn, trace};

use crate::scraper::EntryToSend;

pub(crate) struct SenderLoop<'a> {
    rx: Receiver<EntryToSend>,
    sink: &'a mut SplitSink<WebSocketStream<TcpStream>, Message>,
}

impl<'a> SenderLoop<'a> {
    pub(crate) fn new(
        rx: Receiver<EntryToSend>,
        sink: &'a mut SplitSink<WebSocketStream<TcpStream>, Message>,
    ) -> Self {
        Self { rx, sink }
    }

    pub(crate) async fn start(&mut self) {
        let backoff = Backoff::new();
        loop {
            let entry_result = self.rx.recv().await;
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::Entry(entry) => {
                        backoff.reset();
                        let span = info_span!("Receiving entries to broadcast");
                        let json_string =
                            serde_json::to_string(&entry).expect("Serializing failed!");

                        match self.sink.send(Message::Text(json_string)).await {
                            Ok(_) => {
                                span.in_scope(|| {
                                    trace!("Entry sended!: {}", entry.htmlId);
                                });
                            }
                            Err(err) => {
                                let error_span = error_span!("Receiving entries to broadcast");
                                error_span.in_scope(|| {
                                    error!("Sending failed!: {}", err);
                                });
                            }
                        }
                    }

                    EntryToSend::HypervisorFinish(text) => {
                        backoff.reset();
                        let span = info_span!("Receiving entries to broadcast");

                        match self.sink.send(Message::Text(text.to_string())).await {
                            Ok(_) => {
                                span.in_scope(|| {
                                    info!("`finish` cmd sended!");
                                });
                            }
                            Err(err) => {
                                let error_span = error_span!("Receiving entries to broadcast");
                                error_span.in_scope(|| {
                                    error!("Sending failed!: {}", err);
                                });
                            }
                        }
                    }
                    EntryToSend::Quit => {
                        backoff.reset();
                        let span = info_span!("Receiving entries to broadcast");
                        span.in_scope(|| {
                            info!("Closing scraper thread!");
                        });
                        std::process::exit(0);
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
