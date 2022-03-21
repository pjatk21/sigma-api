#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::{error::Error, sync::Arc};

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
    pub MANAGER_URL:&'static str,
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
            MANAGER_URL: "MANAGER_URL",
            
        }
    }
}

pub(crate) struct Config {
    
    client_webdriver: Arc<WebDriver>,
    
}

impl Config {
    pub(crate) async fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client_webdriver: Arc::new(Config::init_pjatk_client().await?),
        })
    }
    pub fn get_webdriver(&self) -> &Arc<WebDriver> {
        &self.client_webdriver
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
