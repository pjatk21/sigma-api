use std::time::Duration;

use crossbeam::utils::Backoff;
use futures::{stream::SplitStream, StreamExt};
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};
use tracing::{error, error_span, info, info_span, trace};

use crate::{api::HypervisorCommand, scraper::EntryToSend};

pub(crate) struct ReceiverLoop<'a> {
    tx: Sender<EntryToSend>,
    stream: &'a mut SplitStream<WebSocketStream<TcpStream>>,
}

impl<'a> ReceiverLoop<'a> {
    pub(crate) fn new(
        tx: Sender<EntryToSend>,
        stream: &'a mut SplitStream<WebSocketStream<TcpStream>>,
    ) -> Self {
        Self { tx, stream }
    }
    pub(crate) async fn start(&mut self) {
        let span = info_span!("Receiving WebSocket data");
        let backoff = Backoff::new();
        while let Some(resp) = self.stream.next().await {
            trace!("Message received!");
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
