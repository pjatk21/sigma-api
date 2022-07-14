#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::error::Error;
use std::time::Duration;

use crate::loops::parser_loop::ParserLoop;
use crate::loops::{receiver_loop::ReceiverLoop, sender_loop::SenderLoop};

use futures::future::select;
use futures::pin_mut;
use futures::stream::{SplitSink, SplitStream};
use reqwest::IntoUrl;
use tokio::signal::unix::SignalKind;
use tokio::{net::TcpStream, sync::broadcast::Sender};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::scraper::EntryToSend;

pub(crate) struct EventLoop<'a, T: AsRef<str>> {
    receiver: ReceiverLoop<'a>,
    sender: SenderLoop<'a>,
    parser: ParserLoop<'a, T>,
}

impl<'a, T: AsRef<str> + IntoUrl> EventLoop<'a, T> {
    pub(crate) async fn new(
        tx: Sender<EntryToSend>,
        stream: &'a mut SplitStream<WebSocketStream<TcpStream>>,
        sink: &'a mut SplitSink<WebSocketStream<TcpStream>, Message>,
        client: &'a reqwest::Client,
        url: T,
        timeout: Duration,
        max_concurrent: usize,
    ) -> Result<EventLoop<'a, T>, Box<dyn Error>> {
        Ok(Self {
            receiver: ReceiverLoop::new(tx.clone(), stream),
            sender: SenderLoop::new(tx.subscribe(), sink),
            parser: ParserLoop::new(tx.clone(), client, url, timeout, max_concurrent).await?,
        })
    }

    pub(crate) async fn start(&mut self, tx: Sender<EntryToSend>) {
        let receiver = self.receiver.start();
        let sender = self.sender.start();
        let parser = self.parser.start();
        // TODO: Split shutdown thread to seperate struct
        let shutdown = async {
            loop {
                match tokio::signal::unix::signal(SignalKind::terminate())
                    .unwrap()
                    .recv()
                    .await
                {
                    Some(_) => {
                        tx.send(EntryToSend::Quit);
                    }
                    None => {
                        tokio::time::sleep(Duration::from_nanos(250)).await;
                    }
                }
            }
        };

        pin_mut!(receiver);
        pin_mut!(sender);
        pin_mut!(parser);
        pin_mut!(shutdown);

        select(select(receiver, sender), select(parser, shutdown)).await;
    }
}
