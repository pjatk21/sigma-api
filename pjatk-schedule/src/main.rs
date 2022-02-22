use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[allow(unused_imports)]
use futures::stream::TryStreamExt;

use kuchiki::traits::TendrilSink;
use mongodb::{options::ClientOptions, Client, Collection};
use poem::{listener::TcpListener, web::Data, EndpointExt, Route, Server};
use poem_openapi::{param::Path, payload::PlainText, OpenApi, OpenApiService};
use timetable::TimeTableEntry;

use std::{error::Error, sync::Arc, time::Duration};
use thirtyfour::{prelude::*, PageLoadStrategy};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let coll_db = connect_db().await?;
    let client = Arc::new(init_pjatk_client().await?);
    let api_service =
        OpenApiService::new(Api, "pjatk_schedule", "0.1").server("http://127.0.0.1:3002/api");
    let docs = api_service.swagger_ui();
    let app = Route::new()
        .nest("/api", api_service)
        .nest("/", docs)
        .data(coll_db.clone())
        .data(client.clone());
    Server::new(TcpListener::bind("127.0.0.1:3002"))
        .run(app)
        .await?;
    Ok(())
}

struct Api;
#[OpenApi]
impl Api {
    #[oai(path = "/fetch_day/:date", method = "get")]
    async fn fetch_day(
        &self,
        db_client: Data<&Client>,
        web_driver: Data<&Arc<WebDriver>>,
        date: Path<String>,
    ) -> PlainText<String> {
        parse_timetable_day(&web_driver, date.to_string(), db_client.clone())
            .await
            .map_err(|err| eprintln!("{}", &err))
            .expect("failed!");
        PlainText("done".to_string())
    }
}

async fn connect_db() -> Result<Client, Box<dyn Error>> {
    let url = format!(
        "mongodb://{0}:{1}@mongodb:27017",
        std::env::var("MONGO_INITDB_ROOT_USERNAME")?,
        std::env::var("MONGO_INITDB_ROOT_PASSWORD")?
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
    db_client: Client,
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
    let good_elements = table
        .find_elements(By::Css("tbody td[id*=\";r\"],tbody td[id*=\";z\"]"))
        .await?;

    let count = good_elements.len();
    dbg!(format!("Found {} timetable entries", count));
    let window_rect = web_driver.get_window_rect().await?;
    for (index, element) in good_elements.iter().enumerate() {
        let (x, y) = element.rect().await?.icenter();
        if x > window_rect.x || y > window_rect.y || x < 0 || y < 0 {
            element.scroll_into_view().await?;
        }
        web_driver
            .action_chain()
            .move_to_element_center(element)
            .perform()
            .await?;
        let tooltip_element = web_driver
            .query(By::Id("RadToolTipManager1RTMPanel"))
            .wait(Duration::MAX, Duration::from_nanos(125))
            .and_displayed()
            .first()
            .await?;
        let html = tooltip_element.inner_html().await?;
        let tooltip_node = kuchiki::parse_html().from_utf8().one(html.as_bytes());
        let entry: timetable::TimeTableEntry = tooltip_node.try_into()?;
        let client_db = db_client.clone();
        tokio::spawn(async move {
            let db = client_db.database("schedule");
            let timetable: Collection<TimeTableEntry> = db.collection("timetable_entries");

            timetable
                .insert_one(entry, None)
                .await
                .expect("Insert failed!");
        });
        dbg!(index);
    }
    Ok(())
}
