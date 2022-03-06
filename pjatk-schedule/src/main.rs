#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use crate::scraper::EntryToSend;
use api::{Api, ApiError};
use config::Config;
use config::ENVIROMENT;
use mongodb::Collection;
use poem::{
    listener::TcpListener, middleware::TowerLayerCompatExt, Endpoint, EndpointExt, IntoResponse,
    Request, Response, Result, Route, Server,
};
use poem_openapi::OpenApiService;

use timetable::TimeTableEntry;

use std::{error::Error, time::Duration};

mod api;
mod api_response;
mod config;
mod scraper;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new().await?;
    //
    let port = config.get_port();
    let server_url = config.get_complete_server_url();

    let api_service =
        OpenApiService::new(Api, "PJATK Schedule Scrapper API", "0.2").server(server_url);
    let docs = api_service.swagger_ui();
    let open_api_specs = api_service.spec_endpoint();
    //
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<EntryToSend>();
    let tx_clone = tx.clone();
    let client = config.get_webdriver().clone();
    //
    let app = Route::new()
        .nest("/", docs)
        .nest("/api", api_service)
        .nest("/openapi.json", open_api_specs)
        .data(client.clone())
        .data(tx.clone())
        .with(tower::limit::ConcurrencyLimitLayer::new(1).compat())
        .with(tower::buffer::BufferLayer::new(100).compat())
        .catch_all_error(move |err| {
            tx_clone.send(EntryToSend::Quit).expect("quitting failed!");
            custom_err(err)
        })
        .around(auth);
    //
    let coll_db = config.get_db().clone();
    tokio::spawn(async move {
        loop {
            if let Some(entry) = rx.recv().await {
                match entry {
                    EntryToSend::Entry(entry) => {
                        let db = coll_db.database(
                            &std::env::var(ENVIROMENT.MONGO_INITDB_DATABASE)
                                .expect("Missing env: default database"),
                        );
                        let timetable: Collection<TimeTableEntry> = db.collection(
                            &std::env::var(ENVIROMENT.MONGO_INITDB_COLLECTION)
                                .expect("Missing env: default collection"),
                        );

                        timetable
                            .insert_one(entry, None)
                            .await
                            .expect("Insert failed!");
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

async fn custom_err(err: poem::Error) -> impl IntoResponse {
    poem::error::InternalServerError(ApiError {
        cause: err.to_string(),
    })
    .as_response()
}

async fn auth<E: Endpoint>(endpoint: E, request: Request) -> Result<Response> {
    if let Some(auth_code) = request.header("Authorization") {
        if let Ok(auth_key) = std::env::var(ENVIROMENT.AUTH_KEY) {
            if format!("Bearer {}", auth_key) == auth_code {
                let res = endpoint.call(request).await;
                match res {
                    Ok(resp) => {
                        let resp = resp.into_response();
                        Ok(resp)
                    }
                    Err(err) => Err(err),
                }
            } else {
                Err(poem::error::Unauthorized(ApiError {
                    cause: "Bad token".to_string(),
                }))
            }
        } else {
            panic!("No auth key provided! Restart GeckoDriver Docker container!");
        }
    } else {
        Err(poem::error::Unauthorized(ApiError {
            cause: "Missing token".to_string(),
        }))
    }
}
