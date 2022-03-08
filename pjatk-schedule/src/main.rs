#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use crate::scraper::EntryToSend;
use api::Api;
use api_utils::SigmaApiError;

use auth::BearerAuth;
use config::{Config, ENVIROMENT};

use poem::{
    listener::TcpListener, middleware::TowerLayerCompatExt, EndpointExt, Result, Route, Server,
};
use poem_openapi::OpenApiService;

use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use std::{error::Error, time::Duration};

mod api;

mod auth;
mod config;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new().await?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let port = config.get_port();
    let server_url = config.get_complete_server_url();

    let api_service =
        OpenApiService::new(Api, "PJATK Schedule Scrapper API", "0.4.2").server(server_url);
    let docs = api_service.swagger_ui();
    let open_api_specs = api_service.spec_endpoint();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<EntryToSend>();
    let client = config.get_webdriver().clone();
    let upload_client = reqwest::Client::new();
    let app = Route::new()
        .nest("/", docs)
        .nest("/api", api_service)
        .nest("/openapi.json", open_api_specs)
        .data(client.clone())
        .data(tx.clone())
        .with(tower::limit::ConcurrencyLimitLayer::new(1).compat())
        .with(tower::buffer::BufferLayer::new(100).compat())
        .with(poem::middleware::Tracing)
        .with(BearerAuth::new())
        .catch_all_error(SigmaApiError::handle_error);

    let client_db = config.get_db().clone();
    let client_url = upload_client.clone();
    tokio::spawn(async move {
        loop {
            if let Some(entry) = rx.recv().await {
                match entry {
                    EntryToSend::Entry(entry) => {
                        let url = format!(
                            "{0}/v1/timetable/parse",
                            std::env::var(ENVIROMENT.ALTAPI_URL).expect("No Altapi URL found!")
                        );
                        client_url
                            .post(&url)
                            .json(&entry)
                            .header("X-Upload-Key", std::env::var(ENVIROMENT.ALTAPI_KEY).expect("No Aliapi Auth_key found!"))
                            .send()
                            .await
                            .expect("Sending failed!");
                    }
                    EntryToSend::Quit => {
                        client
                            .close()
                            .await
                            .expect("Error closing browser! Restart GeckoDriver Docker container!");
                        break;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });
    Server::new(TcpListener::bind(format!("0.0.0.0:{}", port)))
        .run(app)
        .await?;

    Ok(())
}
