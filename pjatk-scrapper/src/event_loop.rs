use crate::loops::{receiver_loop::ReceiverLoop, sender_loop::SenderLoop};
use crate::loops::parser_loop::ParserLoop;
use std::sync::Arc;

use futures::stream::{SplitSink, SplitStream};
use thirtyfour::WebDriver;
use tokio::{
    net::TcpStream,
    sync::broadcast::Sender,
};
use tokio_tungstenite::{tungstenite::Message, WebSocketStream};

use crate::scraper::EntryToSend;

pub(crate) struct EventLoop<'a> {
    receiver: ReceiverLoop<'a>,
    sender: SenderLoop<'a>,
    parser: ParserLoop,
}

impl<'a> EventLoop<'a> {
    pub(crate) fn new(
        tx: Sender<EntryToSend>,
        stream: &'a mut SplitStream<WebSocketStream<TcpStream>>,
        sink: &'a mut SplitSink<WebSocketStream<TcpStream>, Message>,
        client: Arc<WebDriver>,
    ) -> Self {
        Self {
            receiver: ReceiverLoop::new(tx.clone(), stream),
            sender: SenderLoop::new(tx.clone(), sink),
            parser: ParserLoop::new(tx, client),
        }
    }

    pub(crate) async fn start(&mut self) {
        futures::join!(
            self.receiver.start(),
            self.sender.start(),
            self.parser.start()
        );
    }
}
