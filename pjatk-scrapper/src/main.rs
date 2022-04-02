#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use config::{Config, ENVIROMENT};

use event_loop::EventLoop;
use futures::StreamExt;

use tokio::net::TcpStream;

use tracing::{error, error_span, warn};
use tracing_subscriber::{fmt::format, EnvFilter};

use std::{error::Error, time::Duration};

use crate::scraper::EntryToSend;

mod api;
mod config;
mod event_loop;
mod loops;
mod request;
mod scraper;

static RETRY: u32 = 10;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    std::panic::set_hook(Box::new(move |panic| {
        let error_span = error_span!("Program panics!");
        error_span.in_scope(|| {
            error!("Panic: {:?}", panic);
        });
    }));

    let config = Config::new().await?;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .event_format(format().pretty())
        .init();

    let (tx, _) = tokio::sync::broadcast::channel::<EntryToSend>(500);

    let url = std::env::var(ENVIROMENT.MANAGER_URL).expect("No Altapi URL found!");
    let stream: TcpStream;
    let mut count: u32 = 0;

    loop {
        match TcpStream::connect(&url.replace("ws://", "")).await {
            Ok(stream_result) => {
                stream = stream_result;
                break;
            }
            Err(_) => {
                if count >= RETRY {
                    panic!("Can't connect after 5 tries!");
                } else {
                    warn!("Can't connect, repeating after 1 second...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    count += 1;
                }
            }
        }
    }

    let (websocket, _) = tokio_tungstenite::client_async(url, stream)
        .await
        .expect("WebSocket upgrade failed!");
    let (mut sink, mut stream) = websocket.split();
    let client = config.get_http_client();
    let mut looping = EventLoop::new(tx, &mut stream, &mut sink, client, config.get_url()).await?;

    looping.start().await;

    Ok(())
}
