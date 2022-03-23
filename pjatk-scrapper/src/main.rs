#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]


use crate::scraper::EntryToSend;

use api::HypervisorCommand;
use async_broadcast::TryRecvError;
use async_std::net::TcpStream;
use chrono::{DateTime};
use config::{Config, ENVIROMENT};
use futures::{StreamExt, SinkExt, TryFutureExt};
use scraper::parse_timetable_day;
use async_tungstenite::tungstenite::Message;
use thirtyfour::{Keys, By};
use tracing::{Level, info_span, error_span, error, info, warn};

use std::{error::Error, time::Duration};

mod api;
mod config;
mod scraper;

static RETRY: u32 = 5;

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    std::panic::set_hook(Box::new(move |panic| {
        let error_span = error_span!("Program panics!");
        error_span.in_scope(|| {
            error!("Panic: {:?}", panic);
        });
    }));

    
    let config = Config::new().await?;

    tracing_subscriber::fmt().with_max_level(Level::TRACE).init();

    let (_tx, mut rx) = async_broadcast::broadcast::<EntryToSend>(32);

    rx.set_overflow(true);

    let client = config.get_webdriver().clone();
    
    let url = std::env::var(ENVIROMENT.MANAGER_URL).expect("No Altapi URL found!");
    let stream: TcpStream;
    let mut count:u32 = 0;
    
    
    loop {       
        match async_std::net::TcpStream::connect(&url.replace("ws://","")).await {
            Ok(stream_result) => {
                stream = stream_result;
                break;
            },
            Err(_) => {
                if count >= RETRY {
                    panic!("Can't connect after 5 tries!");
                } else {
                    warn!("Can't connect, repeating after 1 second...");
                    async_std::task::sleep(Duration::from_secs(1)).await;
                    count+=1;
                }
                
            },
        }
    }

    let (websocket,_) = async_tungstenite::client_async(&url,stream).await.expect("WebSocket upgrade failed!");
    let (mut sink, mut stream) = websocket.split();

    
    // Receiving thread
    let tx_command = rx.new_sender();
    let receive_handle = async_std::task::spawn(async move {
        let span = info_span!("Receiving WebSocket data");
        while let Some(resp) = stream.next().await {
            match resp {
                Ok(msg) => match msg {
                    Message::Text(json_str) => {
                        let cmd: HypervisorCommand = serde_json::from_str(&json_str).expect("Parsing failed!");
                        tx_command.try_broadcast(EntryToSend::HypervisorCommand(cmd)).expect("Sending command failed!");
                    },
                    Message::Close(Some(close_frame)) => {
                        span.in_scope(|| {
                            info!("Closing: {}", close_frame);
                        });
                        tx_command.try_broadcast(EntryToSend::Quit).expect("Closing failed!");
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

    let tx_send = rx.new_sender();
    let mut rx_send = rx.new_receiver();
    // Sending thread
    let send_handle = async_std::task::spawn(async move {
        loop {
            let entry_result = rx_send.try_recv();
            match entry_result {
                Ok(entry) =>             match entry {
                    EntryToSend::Entry(entry) => {
                        let span = info_span!("Receiving entries to broadcast");   
                        let json_string = serde_json::to_string(&entry).expect("Serializing failed!");
                        match sink.send(Message::Text(json_string)).await {
                            Ok(_) => {
                                span.in_scope(|| {
                                    info!("Entry sended!: {}", entry.htmlId);
                                });
                            }
                            Err(err) => {
                                let error_span = error_span!("Receiving entries to broadcast");
                                error_span.in_scope(|| {
                                    error!("Sending failed!: {}", err);
                                });
                            }
                        }
                    },
                    EntryToSend::HypervisorFinish(text) => {
                        let span = info_span!("Receiving entries to broadcast");   
                        match sink.send(Message::Text(text.to_string())).await {
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
                        let span = info_span!("Receiving entries to broadcast");
                        span.in_scope(|| {
                            info!("Closing scraper thread!");
                        });
                        break;
                    }
                    _ => {}
                }
            
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Overflowed(_) => {warn!("Receiver overflow!")},
                        TryRecvError::Empty => {},
                        TryRecvError::Closed => {break;},
                    }       
                },
            }
        }
    });

    // Parsing thread
    let parse_handle = async_std::task::spawn(async move {
        loop {
            let entry_result = rx.try_recv();
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::HypervisorCommand(hypervisor_command) => {
                        let date = DateTime::parse_from_rfc3339(&hypervisor_command.scrapUntil).expect("Bad DateTime format until!");
                        let date_str = date.format("%Y-%m-%d").to_string();
                        parse_timetable_day(&client,date_str,rx.new_sender()).and_then(|_| async {
                            let span = info_span!("Parsing entries");
                            span.in_scope(|| {
                                info!("Scrapping ended!: {}",date);
                            });
                            
                            tx_send.try_broadcast(EntryToSend::HypervisorFinish("finished")).expect("`finish`-ing failed!");
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
                Err(try_recv_error) => {
                    match try_recv_error {
                        TryRecvError::Overflowed(_) => {warn!("Receiver overflow!")},
                        TryRecvError::Empty => {},
                        TryRecvError::Closed => {break;},
                    }       
                },
            }
        }
    });
    futures::join!(receive_handle,send_handle,parse_handle);
    Ok(())
}
