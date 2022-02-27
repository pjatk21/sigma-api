#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

#[allow(unused_imports)]
use futures::stream::TryStreamExt;

use kuchiki::traits::TendrilSink;
use mongodb::{options::ClientOptions, Client, Collection};
use poem::{
    listener::TcpListener, web::Data, Endpoint, EndpointExt, IntoResponse, Request, Response,
    Result, Route, Server,
};
use poem_openapi::{param::Path, payload::PlainText, OpenApi, OpenApiService};
use timetable::TimeTableEntry;
use tokio::sync::mpsc::UnboundedSender;

use std::{error::Error, fmt::Display, sync::Arc, time::Duration};
use thirtyfour::{prelude::*, PageLoadStrategy};

#[derive(Debug)]
enum EntryToSend {
    Entry(Box<TimeTableEntry>),
    Quit,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let coll_db = connect_db().await?;
    let client = Arc::new(init_pjatk_client().await?);
    let port = std::env::var("PJATK_SCRAPPER_PORT")?;
    let server_url = format!(
        "{0}:{1}/api",
        std::env::var("PJATK_API_URL_WITH_PROTOCOL")?,
        port
    );
    let api_service =
        OpenApiService::new(Api, "PJATK Schedule Scrapper API", "0.2").server(server_url);
    let docs = api_service.swagger_ui();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<EntryToSend>();
    let tx_clone = tx.clone();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", docs)
        .data(client.clone())
        .data(tx.clone())
        .catch_all_error(move |err| {
            tx_clone.send(EntryToSend::Quit).expect("quitting failed!");
            eprintln!("{:#?}",err);
            custom_err
        }())
        .around(rate_limit_and_auth);
    tokio::spawn(async move {
        loop {
            if let Some(entry) = rx.recv().await {
                match entry {
                    EntryToSend::Entry(entry) => {
                        let db = coll_db.database(
                            &std::env::var("MONGO_INITDB_DATABASE")
                                .expect("Missing env: default database"),
                        );
                        let timetable: Collection<TimeTableEntry> = db.collection(
                            &std::env::var("MONGO_INITDB_COLLECTION")
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

async fn custom_err() -> impl IntoResponse {
    poem::error::InternalServerError(ApiError {
        cause: "Internal Server Error",
    })
    .as_response()
}

async fn rate_limit_and_auth<E: Endpoint>(endpoint: E, request: Request) -> Result<Response> {
    // let ip = request.remote_addr();
    if let Some(auth_code) = request.header("Authorization") {
        if let Ok(auth_key) = std::env::var("AUTH_KEY") {
            if format!("Bearer: {}", auth_key) != auth_code {
                Err(poem::error::Unauthorized(ApiError { cause: "Bad token" }))
            } else {
                let res = endpoint.call(request).await;
                match res {
                    Ok(resp) => {
                        let resp = resp.into_response();
                        Ok(resp)
                    }
                    Err(err) => Err(err),
                }
            }
        } else {
            panic!("No auth key provided! Restart GeckoDriver Docker container!");
        }
    } else {
        Err(poem::error::Unauthorized(ApiError {
            cause: "Missing token",
        }))
    }
}

#[derive(Debug)]
struct ApiError {
    cause: &'static str,
}

impl Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::write(f, format_args!("API error: {}", self.cause))
    }
}

impl Error for ApiError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}

struct Api;
#[OpenApi]
impl Api {
    #[oai(path = "/fetch_days/:beginning_date/:amount_of_days", method = "get")]
    async fn fetch_days(
        &self,
        web_driver: Data<&Arc<WebDriver>>,
        tx: Data<&UnboundedSender<EntryToSend>>,
        beginning_date: Path<String>,
        amount_of_days: Path<Option<u8>>,
    ) -> PlainText<String> {
        let date = chrono::NaiveDate::parse_from_str(&beginning_date.0, "%Y-%m-%d");
        if date.is_err() {
            return PlainText("parsing error".to_string());
        } else {
            let checked_beginning = date.unwrap();
            if let Some(amount_of_days) = amount_of_days.0 {
                for date in checked_beginning.iter_days().take(amount_of_days.into()) {
                    let date_string = date.format("%Y-%m-%d").to_string();
                    web_driver.refresh().await.expect("refresh failed!");
                    parse_timetable_day(&web_driver, date_string, tx.clone())
                        .await
                        .map_err(|err| eprintln!("{}", &err))
                        .expect("failed!");
                }
            } else {
                let date_string = checked_beginning.format("%Y-%m-%d").to_string();
                parse_timetable_day(&web_driver, date_string, tx.clone())
                    .await
                    .map_err(|err| eprintln!("{}", &err))
                    .expect("failed!");
            }
        }
        tx.send(EntryToSend::Quit)
            .expect("Error closing browser! Restart GeckoDriver Docker container!");
        PlainText("done".to_string())
    }
}

async fn connect_db() -> Result<Client, Box<dyn Error>> {
    let url = format!(
        "mongodb://{0}:{1}@{2}:{3}",
        std::env::var("MONGO_INITDB_ROOT_USERNAME")?,
        std::env::var("MONGO_INITDB_ROOT_PASSWORD")?,
        std::env::var("MONGO_HOST")?,
        std::env::var("MONGO_PORT")?,
    );
    let mut client_options = ClientOptions::parse(url).await.expect("Bad mongo url!");
    client_options.app_name = Some("PJATK Schedule".to_string());
    let client_db = Client::with_options(client_options).expect("Client failed!");
    Ok(client_db)
}

async fn init_pjatk_client() -> Result<WebDriver, Box<dyn Error>> {
    let mut caps = DesiredCapabilities::firefox();
    caps.set_headless()?;
    caps.set_page_load_strategy(PageLoadStrategy::None)?;
    let client = WebDriver::new("http://geckodriver:4444", &caps).await?;
    client
        .get("https://planzajec.pjwstk.edu.pl/PlanOgolny3.aspx")
        .await?;
    Ok(client)
}

async fn parse_timetable_day(
    web_driver: &WebDriver,
    date: String,
    tx: UnboundedSender<EntryToSend>,
) -> Result<(), Box<dyn Error>> {
    let date_input = web_driver
        .find_element(By::Id("DataPicker_dateInput"))
        .await?;

    date_input.click().await?;
    date_input.send_keys(Keys::Control + "a").await?;
    date_input.send_keys(date.clone()).await?;
    date_input.send_keys(Keys::Enter).await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let table = web_driver.find_element(By::Id("ZajeciaTable")).await?;
    let good_elements = table.find_elements(By::Css("tbody td[id*=\";\"]")).await?;

    let count = good_elements.len();
    dbg!(format!("Found {} timetable entries", count));
    let window_rect = web_driver.get_window_rect().await?;
    for (index, element) in good_elements.iter().enumerate() {
        let (x, y) = element.rect().await?.icenter();
        if x > window_rect.x || y > window_rect.y || x < 0 || y < 0 {
            element.scroll_into_view().await?;
        }
        element.wait_until().clickable().await?;
        if let Err(err) = web_driver
            .action_chain()
            .move_to_element_center(element)
            .click()
            .perform()
            .await
        {
            eprintln!("Unexpected error: {:#?}", err);
            break;
        }
        let tooltip_element = web_driver
            .query(By::Id("RadToolTipManager1RTMPanel"))
            .wait(Duration::MAX, Duration::from_nanos(125))
            .and_displayed()
            .first()
            .await?;
        let html = tooltip_element.inner_html().await?;
        let tooltip_node = kuchiki::parse_html().from_utf8().one(html.as_bytes());
        let entry: timetable::TimeTableEntry = tooltip_node.try_into()?;
        tx.send(EntryToSend::Entry(Box::new(entry)))?;
        dbg!(index);
    }
    Ok(())
}
