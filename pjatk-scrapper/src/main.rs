#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use crate::scraper::EntryToSend;

use config::{Config, ENVIROMENT};

use rust_socketio::{Client, ClientBuilder, Payload};
// use tracing::error;
// use tracing::error_span;
// use tracing::info;
// use tracing::info_span;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use std::error::Error;

mod api;
mod config;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new().await?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let (tx, mut rx) = tokio::sync::broadcast::channel::<EntryToSend>(32);
    let client = config.get_webdriver().clone();
    let url = std::env::var(ENVIROMENT.ALTAPI_URL).expect("No Altapi URL found!");
    let fetch_days = |payload: Payload, socket: Client| match payload {
        Payload::Binary(_) => todo!(),
        Payload::String(_) => todo!(),
    };
    let connect = |payload: Payload, socket: Client| match payload {
        Payload::Binary(_) => todo!(),
        Payload::String(_) => todo!(),
    };
    let disconnect = |payload: Payload, socket: Client| match payload {
        Payload::Binary(_) => todo!(),
        Payload::String(_) => todo!(),
    };
    // TODO: Waiting on Socket.io API schema
    let socket_io = ClientBuilder::new(url)
        .on("connect", connect)
        .on("fetch_days", fetch_days)
        .on("disconnect", disconnect)
        .connect()?;

    // let (mut sink, mut stream) = websocket.split();
    // // Receiving thread
    // tokio::spawn(async move {
    //     let span = info_span!("Receiving WebSocket data");
    //     while let Some(resp) = stream.next().await {
    //         match resp {
    //             Ok(msg) => match msg {
    //                 // TODO: Waiting on WebSocket API Schema
    //                 Message::Text(json_str) => {
    //                     todo!()
    //                 },
    //                 Message::Binary(json_vec) => {
    //                     todo!()
    //                 },
    //                 Message::Ping(payload) => {
    //                     match tx.send(WebSocketMessage(Message::Pong(payload))) {
    //                         Ok(_) => {
    //                             span.in_scope(|| {
    //                                 info!("Ping Pong!");
    //                             });
    //                         }
    //                         Err(err) => {
    //                             let error_span = error_span!("Receiving WebSocket data");
    //                             error_span.in_scope(|| {
    //                                 error!("Error: {}", err);
    //                             });
    //                         }
    //                     }
    //                 }
    //                 Message::Close(Some(close_frame)) => {
    //                     span.in_scope(|| {
    //                         info!("Closing: {}", close_frame);
    //                     });
    //                 }
    //                 _ => {}
    //             },
    //             Err(err) => {
    //                 let error_span = error_span!("Receiving WebSocket data");
    //                 error_span.in_scope(|| {
    //                     error!("Error: {}", err);
    //                 });
    //             }
    //         }
    //     }
    // });
    // // Sending thread
    // tokio::spawn(async move {
    //     while let Ok(entry) = rx.recv().await {
    //         match entry {
    //             EntryToSend::Entry(entry) => {
    //                 let span = info_span!("Receiving entries to send");
    //                 /*
    //                  TODO: Assuming data would be send as binary - waiting on schema to confirm...
    //                  (if data would be send as a string)

    //                     ```rust
    //                     let json_string = serde_json::to_string(&entry).expect("Serializing failed!");
    //                     match sink.send(Message::Text(json_string)).await {todo!()}
    //                     ```
    //                 */
    //                 let json = serde_json::to_vec(&entry).expect("Serializing failed!");
    //                 match sink.send(Message::Binary(json)).await {
    //                     Ok(_) => {
    //                         span.in_scope(|| {
    //                             info!("Entry sended!: {}", entry.html_id);
    //                         });
    //                     }
    //                     Err(err) => {
    //                         let error_span = error_span!("Receiving entries to send");
    //                         error_span.in_scope(|| {
    //                             error!("Sending failed!: {}", err);
    //                         });
    //                     }
    //                 }
    //             }
    //             EntryToSend::WebSocketMessage(message) => {
    //                 let span = info_span!("Sending WebSocket data");
    //                 match sink.send(message.clone()).await {
    //                     Ok(_) => {
    //                         span.in_scope(|| {
    //                             info!("Message sended!: {}", message);
    //                         });
    //                     }
    //                     Err(err) => {
    //                         let error_span = error_span!("Sending WebSocket dataF");
    //                         error_span.in_scope(|| {
    //                             error!("Sending failed!: {}", err);
    //                         });
    //                     }
    //                 }
    //             }
    //             EntryToSend::Quit => {
    //                 let span = info_span!("Receiving entries to send");
    //                 span.in_scope(|| {
    //                     info!("Closing scraper thread!");
    //                 });
    //                 break;
    //             }
    //         }
    //     }
    // });
    Ok(())
}
