use std::time::Duration;

use crossbeam::utils::Backoff;
use futures::{stream::SplitStream, StreamExt};
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tracing::{info_span, info, error_span, error};

use crate::{scraper::EntryToSend, api::HypervisorCommand};


pub(crate) struct ReceiverLoop {
    tx: Sender<EntryToSend>,
    stream: SplitStream<WebSocketStream<TcpStream>>,
}

impl ReceiverLoop {
    pub(crate) fn new(
        tx: Sender<EntryToSend>,
        stream: SplitStream<WebSocketStream<TcpStream>>,
    ) -> Self {
        Self {
            tx,
            stream,
        }
    }
    pub(crate) async fn start(&mut self) {
        let span = info_span!("Receiving WebSocket data");
        let backoff = Backoff::new();
        while let Some(resp) = self.stream.next().await {
            match resp {
                Ok(msg) => match msg {
                    Message::Text(json_str) => {
                        backoff.reset();
                        let cmd: HypervisorCommand =
                            serde_json::from_str(&json_str).expect("Parsing failed!");
                        self.tx
                            .send(EntryToSend::HypervisorCommand(cmd))
                            .expect("Sending command failed!");
                    }
                    Message::Close(Some(close_frame)) => {
                        backoff.reset();
                        span.in_scope(|| {
                            info!("Closing: {}", close_frame);
                        });
                        self.tx.send(EntryToSend::Quit).expect("Closing failed!");
                        break;
                    }
                    _ => {
                        backoff.snooze();
                    }
                },
                Err(err) => {
                    backoff.reset();
                    let error_span = error_span!("Receiving WebSocket data");
                    error_span.in_scope(|| {
                        error!("Error: {}", err);
                    });
                }
            }
            tokio::time::sleep(Duration::from_nanos(250)).await;
        }
    }
}
