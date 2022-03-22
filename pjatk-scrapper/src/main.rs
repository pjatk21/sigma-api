#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]


use crate::scraper::EntryToSend;

use api::HypervisorCommand;
use chrono::{DateTime};
use config::{Config, ENVIROMENT};
use futures::{StreamExt, SinkExt, TryFutureExt};
use scraper::parse_timetable_day;
use thirtyfour::{By, Keys};
use tokio_tungstenite::tungstenite::Message;
use tracing::{Level, info_span, error_span, error, info};

use std::{error::Error, sync::Arc};

mod api;
mod config;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new().await?;

    tracing_subscriber::fmt().with_max_level(Level::TRACE).init();

    let (tx, mut rx) = tokio::sync::broadcast::channel::<EntryToSend>(32);

    let client = config.get_webdriver().clone();
    
    let url = std::env::var(ENVIROMENT.MANAGER_URL).expect("No Altapi URL found!");

    let (websocket,_) = tokio_tungstenite::connect_async(&url).await.expect("WebSocket connection failed!");

    let (mut sink, mut stream) = websocket.split();

    let tx_command = tx.clone();
    let tx_panic = tx.clone();
    std::panic::set_hook(Box::new(move |panic| {
            let error_span = error_span!("Program panics!");
            error_span.in_scope(|| {
                error!("Panic: {}", panic);
            });
            tx_panic.send(EntryToSend::Quit).map_err(|_| {});
        }));
    // Receiving thread
    let receive_handle = tokio::spawn(async move {
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
                        break;
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
    let mut client_close = Arc::clone(&client);
    // Sending thread
    let send_handle = tokio::spawn(async move {
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
                EntryToSend::HypervisorFinish(text) => {
                    let span = info_span!("Receiving entries to send");   
                    match sink.send(Message::Text(text.to_string())).await {
                        Ok(_) => {
                            span.in_scope(|| {
                                info!("`finish` cmd sended!");
                            });
                        }
                        Err(err) => {
                            let error_span = error_span!("Receiving entries to send");
                            error_span.in_scope(|| {
                                error!("Sending failed!: {}", err);
                            });
                        }
                    }
                }
                EntryToSend::Quit => {
                    let span = info_span!("Receiving entries to send");
                    span.in_scope(|| {
                        info!("Closing scraper thread!");
                    });
                    break;
                }
                _ => {}
            }
        }
    });

    // Parsing thread
    let parse_handle = tokio::spawn(async move {
        while let Ok(entry) = rx.recv().await {
            match entry {
                EntryToSend::HypervisorCommand(hypervisor_command) => {
                    let date = DateTime::parse_from_rfc3339(&hypervisor_command.scrapUntil).expect("Bad DateTime format until!");
                    let date_str = date.format("%Y-%m-%d").to_string();
                    parse_timetable_day(&client,date_str,tx.clone()).and_then(|_| async {
                        let span = info_span!("Parsing entries");
                        span.in_scope(|| {
                            info!("Scrapping ended!: {}",date);
                        });
                        
                        tx.send(EntryToSend::HypervisorFinish("finished")).expect("`finish`-ing failed!");
                        client.refresh().await.expect("Refreshing page failed!");
                        Ok(())
                    }).await.expect("Parsing failed!");
                },
                EntryToSend::Quit => {
                    let span = info_span!("Parsing entries");
                    span.in_scope(|| {
                        info!("Closing scraper thread!");
                    });

                    let window = client.find_element(By::Css("html")).await.expect("Find element failed!");
                    window.send_keys(Keys::Alt + Keys::F4).await.expect("Close window failed! Stop geckodriver container manually!");

                    break;
                },
                _ => {}
            }
            

        }
    });
    futures::join!(receive_handle,send_handle,parse_handle);
    Ok(())
}
