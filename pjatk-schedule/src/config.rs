#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::{error::Error, sync::Arc};

use mongodb::{options::ClientOptions, Client};
use thirtyfour::{DesiredCapabilities, PageLoadStrategy, WebDriver};

pub(crate) static ENVIROMENT: Env = Env::new();

#[allow(non_snake_case)]
pub(crate) struct Env {
    pub PJATK_SCRAPPER_PORT: &'static str,
    pub PJATK_SCRAPPER_URL_WITH_PROTOCOL: &'static str,
    pub MONGO_INITDB_ROOT_USERNAME: &'static str,
    pub MONGO_INITDB_ROOT_PASSWORD: &'static str,
    pub MONGO_HOST: &'static str,
    pub MONGO_PORT: &'static str,
    pub MONGO_INITDB_DATABASE: &'static str,
    pub MONGO_INITDB_COLLECTION: &'static str,
    pub ALTAPI_URL:&'static str,
    pub ALTAPI_KEY:&'static str,
    pub AUTH_KEY: &'static str,
}

impl Env {
    pub(crate) const fn new() -> Self {
        Self {
            PJATK_SCRAPPER_PORT: "PJATK_SCRAPPER_PORT",
            PJATK_SCRAPPER_URL_WITH_PROTOCOL: "PJATK_SCRAPPER_URL_WITH_PROTOCOL",
            MONGO_INITDB_ROOT_USERNAME: "MONGO_INITDB_ROOT_USERNAME",
            MONGO_INITDB_ROOT_PASSWORD: "MONGO_INITDB_ROOT_PASSWORD",
            MONGO_HOST: "MONGO_HOST",
            MONGO_PORT: "MONGO_PORT",
            MONGO_INITDB_DATABASE: "MONGO_INITDB_DATABASE",
            MONGO_INITDB_COLLECTION: "MONGO_INITDB_COLLECTION",
            AUTH_KEY: "AUTH_KEY",
            ALTAPI_URL: "ALTAPI_URL",
            ALTAPI_KEY: "ALTAPI_KEY"
        }
    }
}

pub(crate) struct Config {
    client_db: Client,
    client_webdriver: Arc<WebDriver>,
    port: u16,
    server_url_with_protocol: String,
}

impl Config {
    pub(crate) async fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client_db: Config::connect_db().await?,
            client_webdriver: Arc::new(Config::init_pjatk_client().await?),
            port: std::env::var(ENVIROMENT.PJATK_SCRAPPER_PORT)?.parse()?,
            server_url_with_protocol: std::env::var(ENVIROMENT.PJATK_SCRAPPER_URL_WITH_PROTOCOL)?,
        })
    }
    pub fn get_db(&self) -> &Client {
        &self.client_db
    }
    pub fn get_webdriver(&self) -> &Arc<WebDriver> {
        &self.client_webdriver
    }
    pub fn get_complete_server_url(&self) -> String {
        format!("{0}:{1}/api", self.server_url_with_protocol, self.port)
    }
    pub fn get_port(&self) -> u16 {
        self.port
    }
    async fn connect_db() -> Result<Client, Box<dyn Error>> {
        let url = format!(
            "mongodb://{0}:{1}@{2}:{3}",
            std::env::var(ENVIROMENT.MONGO_INITDB_ROOT_USERNAME)?,
            std::env::var(ENVIROMENT.MONGO_INITDB_ROOT_PASSWORD)?,
            std::env::var(ENVIROMENT.MONGO_HOST)?,
            std::env::var(ENVIROMENT.MONGO_PORT)?,
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
}
