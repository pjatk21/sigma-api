#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use api_utils::SigmaApiError;
use api_utils::SigmaApiResponse;
use poem_openapi::payload::Json;
use thirtyfour::WebDriver;

use tokio::sync::mpsc::UnboundedSender;

use poem::web::Data;
use poem_openapi::{param::Path, OpenApi};

use std::sync::Arc;

use crate::scraper::{parse_timetable_day, EntryToSend};

pub(crate) struct Api;
#[OpenApi]
impl Api {
    #[oai(path = "/fetch_days/:beginning_date/:amount_of_days", method = "get")]
    async fn fetch_days(
        &self,
        web_driver: Data<&Arc<WebDriver>>,
        tx: Data<&UnboundedSender<EntryToSend>>,
        beginning_date: Path<String>,
        amount_of_days: Path<Option<u8>>,
    ) -> SigmaApiResponse<String, SigmaApiError> {
        let date = chrono::NaiveDate::parse_from_str(&beginning_date.0, "%Y-%m-%d");
        if date.is_err() {
            return SigmaApiResponse::BadRequest(Json(
                SigmaApiError::error(400, "Parsing error".to_string(), None)
                    .expect("Error failed!"),
            ));
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
        SigmaApiResponse::Found(Json("Done!".to_string()))
    }
}
