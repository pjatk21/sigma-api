#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]


use api::HypervisorCommand;


use chrono::DateTime;
use config::{Config, ENVIROMENT};
use crossbeam::utils::Backoff;
use futures::{StreamExt, SinkExt, TryFutureExt};
use thirtyfour::{Keys, By};
use tokio::{net::TcpStream, sync::broadcast::error::RecvError};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info_span, error_span, error, info, warn};
use tracing_subscriber::EnvFilter;

use std::{error::Error, time::Duration};

use crate::scraper::{EntryToSend, parse_timetable_day};

mod api;
mod config;
mod scraper;

static RETRY: u32 = 5;

#[tokio::main(flavor="multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    std::panic::set_hook(Box::new(move |panic| {
        let error_span = error_span!("Program panics!");
        error_span.in_scope(|| {
            error!("Panic: {:?}", panic);
        });
    }));

    
    let config = Config::new().await?;

    tracing_subscriber::fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let (tx, mut rx) = tokio::sync::broadcast::channel::<EntryToSend>(500);

    let client = config.get_webdriver().clone();
    
    let url = std::env::var(ENVIROMENT.MANAGER_URL).expect("No Altapi URL found!");
    let stream: TcpStream;
    let mut count:u32 = 0;
    
    loop {       
        match TcpStream::connect(&url.replace("ws://","")).await {
            Ok(stream_result) => {
                stream = stream_result;
                break;
            },
            Err(_) => {
                if count >= RETRY {
                    panic!("Can't connect after 5 tries!");
                } else {
                    warn!("Can't connect, repeating after 1 second...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    count+=1;
                }
            },
        }
    }

    let (websocket,_) = tokio_tungstenite::client_async(url,stream).await.expect("WebSocket upgrade failed!");
    let (mut sink, mut stream) = websocket.split();

    
    // Receiving thread
    let tx_command = tx.clone();
    let receive_handle = tokio::task::spawn(async move {
        let span = info_span!("Receiving WebSocket data");
        let backoff = Backoff::new();
        while let Some(resp) = stream.next().await {
            match resp {
                Ok(msg) => match msg {
                    Message::Text(json_str) => {
                        backoff.reset();
                        let cmd: HypervisorCommand = serde_json::from_str(&json_str).expect("Parsing failed!");
                        tx_command.send(EntryToSend::HypervisorCommand(cmd)).expect("Sending command failed!");
                    },
                    Message::Close(Some(close_frame)) => {
                        backoff.reset();
                        span.in_scope(|| {
                            info!("Closing: {}", close_frame);
                        });
                        tx_command.send(EntryToSend::Quit).expect("Closing failed!");
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
    });

    let tx_send = tx.clone();
    let mut rx_send = tx.subscribe();
    // Sending thread
    let send_handle = tokio::task::spawn(async move {
        let backoff = Backoff::new();
        loop {
            let entry_result = rx_send.recv().await;
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::Entry(entry) => {
                        backoff.reset();
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
                        backoff.reset();
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
                        backoff.reset();
                        let span = info_span!("Receiving entries to broadcast");
                        span.in_scope(|| {
                            info!("Closing scraper thread!");
                        });
                        break;
                    }
                    _ => {
                        backoff.snooze();
                    }
                }
            
                Err(recv_error) => {
                    match recv_error {
                        RecvError::Lagged(number) => {
                            warn!("Receiver overflow! Skipping: {}",number);
                            backoff.snooze();
                        },
                        RecvError::Closed => {break;},
                    }       
                },
            }
            tokio::time::sleep(Duration::from_nanos(250)).await;
        }
    });

    // Parsing thread
    let parse_handle = tokio::task::spawn(async move {
        loop {
            let entry_result = rx.recv().await;
            let backoff = Backoff::new();
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::HypervisorCommand(hypervisor_command) => {
                        let date = DateTime::parse_from_rfc3339(&hypervisor_command.scrapUntil).expect("Bad DateTime format until!");
                        let date_str = date.format("%Y-%m-%d").to_string();
                        parse_timetable_day(&client,date_str,tx.clone()).and_then(|_| async {
                            let span = info_span!("Parsing entries");
                            span.in_scope(|| {
                                info!("Scrapping ended!: {}",date);
                            });
                            
                            tx_send.send(EntryToSend::HypervisorFinish("finished")).expect("`finish`-ing failed!");
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
                    _ => {
                        backoff.snooze();
                    }
                }
                Err(recv_error) => {
                    match recv_error {
                        RecvError::Lagged(number) => {
                            warn!("Receiver overflow! Skipping: {}",number);
                            backoff.snooze();
                        },
                        RecvError::Closed => {break;},
                    }       
                },
            }
            tokio::time::sleep(Duration::from_nanos(250)).await;
        }
    });
    if let (Err(a),Err(b),Err(c)) = futures::join!(receive_handle,send_handle,parse_handle) {
        error!("Error joining threads!: {0:?}, {1:?}, {2:?}",a,b,c);
    };
    Ok(())
}
