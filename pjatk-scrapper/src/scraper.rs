#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::error::Error;
use std::fmt::{Debug, Display};


use timetable::altapi_timetable::UploadEntry;
use tokio::sync::broadcast::Sender;

use tracing::{error, info, warn};

use crate::api::HypervisorCommand;

#[derive(Debug, Clone)]
pub enum EntryToSend {
    HypervisorCommand(HypervisorCommand),
    Entry(UploadEntry),
    HypervisorFinish(&'static str),
    Quit,
}

#[tracing::instrument]
pub(crate) async fn parse_timetable_day(
    http_client: &reqwest::Client,
    date: String,
    tx: Sender<EntryToSend>,
) -> Result<(), Box<dyn Error>> {
    
    let good_elements = table.find_elements(By::Css("tbody td[id*=\";\"]")).await?;

    let count = good_elements.len();
    info!("Found {} timetable entries", count);

    let mut faulty_elements_tier_1: Vec<String> = vec![];
    let mut faulty_elements_tier_2: Vec<String> = vec![];

    // Normal scrapping (5-sec. timeout)
    for (index, element) in good_elements.iter().enumerate() {
        let htmlId = element.id().await?.unwrap();
        parse_timetable_entry(
            htmlId,
            http_client,
            date.clone(),
            tx.clone(),
            Some(&mut faulty_elements_tier_1),
            5,
        )
        .await?;
        info!("{}", index);
    }
    // Tier-1 failure scrapping (10-sec. timeout)
    for (index, htmlId) in faulty_elements_tier_1.into_iter().enumerate() {
        parse_timetable_entry(
            htmlId,
            http_client,
            &date,
            tx.clone(),
            Some(&mut faulty_elements_tier_2),
            10,
        )
        .await?;
        info!("{}", index);
    }
    // Tier-2 failure scrapping (30-sec timeout)
    for (index, htmlId) in faulty_elements_tier_2.into_iter().enumerate() {
        parse_timetable_entry(
            htmlId,
            http_client,
            &date,
            tx.clone(),
            None,
            30,
        )
        .await?;
        info!("{}", index);
    }
    Ok(())
}

#[tracing::instrument]
pub(crate) async fn parse_timetable_entry<T>(
    htmlId: String,
    http_client: &reqwest::Client,
    date: T,
    tx: Sender<EntryToSend>,
    faulty_elements: Option<&mut Vec<String>>,
    timeout: u64,
) -> Result<(), Box<WebDriverError>>
where
    T: AsRef<str> + Debug + Display,
{

    let html = tooltip_element.inner_html().await?;
    let entry: UploadEntry = UploadEntry {
        htmlId: htmlId.clone(),
        body: html,
    };
    if let Err(error) = tx.send(EntryToSend::Entry(entry)) {
        if let Some(vec) = faulty_elements {
            warn!("Broadcasting failed, trying again... {}", error);
            vec.push(htmlId);
        } else {
            error!("Broadcasting failed again... {}", error);
        }
    }
    Ok(())
}
