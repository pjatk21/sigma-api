#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::error::Error;

use reqwest::Client;

use crate::loops::parser_loop::ParserLoop;

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
    client_http_client: reqwest::Client,
    url: &'static str,
}

impl Config {
    pub(crate) async fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client_http_client: Config::init_pjatk_client().await?,
            url:"https://planzajec.pjwstk.edu.pl/PlanOgolny3.aspx",
        })
    }
    pub fn get_http_client(&self) -> &Client {
        &self.client_http_client
    }
    pub fn get_url(&self) -> &'static str {
        self.url
    }
    async fn init_pjatk_client() -> Result<reqwest::Client, reqwest::Error> {
        let headers = ParserLoop::<String>::get_base_headers().expect("Default headers fail!");
        reqwest::Client::builder().default_headers(headers).build()
    }
}
