#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]


use crate::scraper::EntryToSend;

use api::HypervisorCommand;
use chrono::NaiveDate;
use config::{Config, ENVIROMENT};
use futures::{StreamExt, SinkExt};
use scraper::parse_timetable_day;
use tokio_tungstenite::tungstenite::Message;
use tracing::{Level, info_span, error_span, error, info};
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

    let url = std::env::var(ENVIROMENT.MANAGER_URL).expect("No Altapi URL found!");

    let (websocket,_) = tokio_tungstenite::connect_async(&url).await?;

    let (mut sink, mut stream) = websocket.split();

    let tx_command = tx.clone();

    // Receiving thread
    tokio::spawn(async move {
        let span = info_span!("Receiving WebSocket data");
        while let Some(resp) = stream.next().await {
            match resp {
                Ok(msg) => match msg {
                    Message::Text(json_str) => {
                        let cmd: HypervisorCommand = serde_json::from_str(&json_str).expect("Parsing failed!");
                        tx_command.send(EntryToSend::HypervisorCommand(cmd)).expect("Sending command failed!");
                    },
                    Message::Close(Some(close_frame)) => {
                        span.in_scope(|| {
                            info!("Closing: {}", close_frame);
                        });
                        tx_command.send(EntryToSend::Quit).expect("Closing failed!");
                        std::process::exit(0);
                    }
                    _ => {}
                },
                Err(err) => {
                    let error_span = error_span!("Receiving WebSocket data");
                    error_span.in_scope(|| {
                        error!("Error: {}", err);
                    });
                    
                }
            }
        }
    });

    let mut rx_send = tx.subscribe();
    // Sending thread
    tokio::spawn(async move {
        while let Ok(entry) = rx_send.recv().await {
            match entry {
                EntryToSend::Entry(entry) => {
                    let span = info_span!("Receiving entries to send");   
                    let json_string = serde_json::to_string(&entry).expect("Serializing failed!");
                    match sink.send(Message::Text(json_string)).await {
                        Ok(_) => {
                            span.in_scope(|| {
                                info!("Entry sended!: {}", entry.htmlId);
                            });
                        }
                        Err(err) => {
                            let error_span = error_span!("Receiving entries to send");
                            error_span.in_scope(|| {
                                error!("Sending failed!: {}", err);
                            });
                        }
                    }
                },
                
                EntryToSend::Quit => {
                    let span = info_span!("Receiving entries to send");
                    span.in_scope(|| {
                        info!("Closing scraper thread!");
                    });
                    std::process::exit(0);
                }
                _ => {}
            }
        }
    });

    // Parsing thread
    tokio::spawn(async move {
        while let Ok(entry) = rx.recv().await {
            match entry {
                EntryToSend::HypervisorCommand(hypervisor_command) => {
                    let date_first = NaiveDate::parse_from_str(&hypervisor_command.scrapStart, "%Y-%m-%d").expect("");
                    for date in date_first.iter_days().skip(hypervisor_command.skip.unwrap_or(0)).take(hypervisor_command.limit.unwrap_or(0)) {
                        let date_str = date.format("%Y-%m-%d").to_string();
                        parse_timetable_day(&client,date_str,tx.clone()).await.expect("");
                    }
                },
                EntryToSend::Quit => {
                    let span = info_span!("Receiving entries to send");
                    span.in_scope(|| {
                        info!("Closing scraper thread!");
                    });
                    std::process::exit(0);
                },
                _ => {}
            }
            

        }
    });

    Ok(())
}
