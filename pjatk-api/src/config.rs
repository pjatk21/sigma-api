use std::{error::Error};

use mongodb::{options::ClientOptions, Client};

pub(crate) static ENVIROMENT: Env = Env::new();

#[allow(non_snake_case)]
pub(crate) struct Env {
    pub PJATK_API_PORT: &'static str,
    pub PJATK_API_URL_WITH_PROTOCOL: &'static str,
    pub MONGO_INITDB_ROOT_USERNAME: &'static str,
    pub MONGO_INITDB_ROOT_PASSWORD: &'static str,
    pub MONGO_HOST: &'static str,
    pub MONGO_PORT: &'static str,
}

impl Env {
    pub(crate) const fn new() -> Self {
        Self {
            PJATK_API_PORT: "PJATK_API_PORT",
            PJATK_API_URL_WITH_PROTOCOL: "PJATK_API_URL_WITH_PROTOCOL",
            MONGO_INITDB_ROOT_USERNAME: "MONGO_INITDB_ROOT_USERNAME",
            MONGO_INITDB_ROOT_PASSWORD: "MONGO_INITDB_ROOT_PASSWORD",
            MONGO_HOST: "MONGO_HOST",
            MONGO_PORT: "MONGO_PORT",
        }
    }
}

pub(crate) struct Config {
    client_db: Client,
    port: u8,
    server_url_with_protocol: String,
}

impl Config {
    pub(crate) async fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            client_db: Config::connect_db().await?,
            port: std::env::var(ENVIROMENT.PJATK_API_PORT)?.parse()?,
            server_url_with_protocol: std::env::var(ENVIROMENT.PJATK_API_URL_WITH_PROTOCOL)?,
        })
    }
    pub fn get_db(&self) -> &Client {
        &self.client_db
    }
    pub fn get_complete_server_url(&self) -> String {
        format!("{0}:{1}", self.server_url_with_protocol, self.port)
    }
    pub fn get_port(&self) -> u8 {
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
}
