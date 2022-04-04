use std::error::Error;
use std::time::Duration;

use crate::loops::parser_loop::ParserLoop;
use crate::loops::{receiver_loop::ReceiverLoop, sender_loop::SenderLoop};



use futures::stream::{SplitSink, SplitStream};
use reqwest::IntoUrl;
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
        timeout: Duration
    ) -> Result<EventLoop<'a, T>, Box<dyn Error>> {
        Ok(Self {
            receiver: ReceiverLoop::new(tx.clone(), stream),
            sender: SenderLoop::new(tx.subscribe(), sink),
            parser: ParserLoop::new(tx.clone(), client, url,timeout).await?,
        })
    }

    pub(crate) async fn start(&mut self, tx: Sender<EntryToSend>) {
        let (future, abort_handle) = futures::future::abortable(futures::future::join3(
            self.receiver.start(),
            self.sender.start(),
            self.parser.start()
        ));
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            abort_handle.abort();
            std::process::exit(0);
        });
        future.await;
    }
}
