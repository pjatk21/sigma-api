#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use regex::bytes::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{Debug, Display};

use timetable::altapi_timetable::UploadEntry;
use tokio::sync::broadcast::Sender;

use tracing::{error, info};

use crate::api::HypervisorCommand;
use crate::loops::parser_loop::ParserLoop;
use lazy_static::lazy_static;

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
    base_validation: &mut HashMap<&'static str, String>,
) -> Result<(), Box<dyn Error>> {
    let date_form = ParserLoop::get_date_form(base_validation.clone(), date.clone()).await;
    let response = http_client
        .post("https://planzajec.pjwstk.edu.pl/PlanOgolny3.aspx")
        .form(&date_form)
        .send()
        .await?;
    lazy_static! {
        static ref HTML_ID_REGEX: Regex = Regex::new(r"\d+;[zr]").unwrap();
    }
    let bytes = response.bytes().await?;
    let mut good_elements = HTML_ID_REGEX.find_iter(bytes.as_ref());

    let count = good_elements.by_ref().count();
    info!("Found {} timetable entries", count);

    // Normal scrapping (5-sec. timeout)
    for (index, html_id) in good_elements.enumerate() {
        let vec = html_id.as_bytes().to_vec();
        let html_str = String::from_utf8(vec)?;
        parse_timetable_entry(
            html_str,
            http_client,
            date.clone(),
            tx.clone(),
            5,
            base_validation,
        )
        .await.expect("");
        info!("{}", index);
    }
    Ok(())
}

#[tracing::instrument]
pub(crate) async fn parse_timetable_entry<T>(
    html_id: String,
    http_client: &reqwest::Client,
    date: T,
    tx: Sender<EntryToSend>,
    timeout: u64,
    base_validation: &mut HashMap<&'static str, String>,
) -> Result<(), Box<dyn Error>>
where
    T: AsRef<str> + Debug + Display,
{
    let timetable_entry_form = ParserLoop::get_parse_form(date, base_validation.clone());
    let response = http_client
        .post("https://planzajec.pjwstk.edu.pl/PlanOgolny3.aspx")
        .form(&timetable_entry_form)
        .send()
        .await?;
    let response_string = String::from_utf8(response.bytes().await?.to_vec())?;

    let mut splitted = response_string.split('|');

    let position_html = splitted.position(|x| x == "RadAjaxPanel1Panel");
    let position_view_state = splitted.position(|x| x == "__VIEWSTATE");
    let position_view_state_generator = splitted.position(|x| x == "__VIEWSTATEGENERATOR");
    let position_event_validation = splitted.position(|x| x == "__EVENTVALIDATION");
    let splitted_vec: Vec<&str> = splitted.collect();
    
    let html = splitted_vec[position_html.unwrap() + 1];
    let view_state = splitted_vec[position_view_state.unwrap() + 1];
    let view_state_generator = splitted_vec[position_view_state_generator.unwrap() + 1];
    let event_validation = splitted_vec[position_event_validation.unwrap() + 1];

    let entry: UploadEntry = UploadEntry {
        htmlId: html_id.clone(),
        body: html.to_string(),
    };
    if let Err(error) = tx.send(EntryToSend::Entry(entry)) {
        error!("Broadcasting failed {}", error);
    }
    *base_validation.get_mut("__VIEWSTATE").unwrap() = view_state.to_string();
    *base_validation.get_mut("__VIEWSTATEGENERATOR").unwrap() = view_state_generator.to_string();
    *base_validation.get_mut("__EVENTVALIDATION").unwrap() = event_validation.to_string();
    Ok(())
}
