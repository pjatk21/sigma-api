#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use regex::Regex;
use reqwest::IntoUrl;
use std::collections::{HashMap, HashSet};
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
    url: String,
) -> Result<(), Box<dyn Error>> {
    let (date_form, bug_check) =
        ParserLoop::get_date_form(base_validation.clone(), date.clone()).await;
    let response = http_client
        .post(url.clone())
        .form(&date_form)
        .send()
        .await?;

    let bytes = response.bytes().await?;
    let html_body = String::from_utf8(bytes.as_ref().to_vec())?;
    let html_string: String = if !bug_check {
        ParserLoop::<String>::update_base_validation_and_give_html_delta(html_body.clone(), base_validation,"RadAjaxPanel1Panel".to_string())
            .await
    } else {
        ParserLoop::<String>::update_base_validation_and_give_html_full(
            html_body.clone(),
            base_validation,
        )
        .await;
        html_body.clone()
    };
    info!("HTML body: {}", html_string);
    lazy_static! {
        static ref HTML_ID_REGEX: Regex = Regex::new(r"\d+;[zr]").unwrap();
    }

    let good_elements: HashSet<&str> = HashSet::from_iter(
        HTML_ID_REGEX
            .find_iter(&html_string)
            .map(|mat| mat.as_str()),
    );

    let count = good_elements.len();
    info!("Found {} timetable entries", count);

    // Normal scrapping (5-sec. timeout)
    for (index, html_id) in good_elements.iter().enumerate() {
        parse_timetable_entry(
            html_id.to_string(),
            http_client,
            date.clone(),
            tx.clone(),
            5,
            base_validation,
            url.clone(),
        )
        .await
        .expect("");
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
    url: String,
) -> Result<(), Box<dyn Error>>
where
    T: AsRef<str> + Debug + Display + IntoUrl,
{
    let timetable_entry_form = ParserLoop::get_parse_form(html_id.clone(), base_validation.clone());
    let response = http_client
        .post(url)
        .form(&timetable_entry_form)
        .send()
        .await?;
    let response_string = String::from_utf8(response.bytes().await?.to_vec())?;

    let html = ParserLoop::<String>::update_base_validation_and_give_html_delta(
        response_string,
        base_validation,
        "RadToolTipManager1RTMPanel".to_string()
    )
    .await;

    let entry: UploadEntry = UploadEntry {
        htmlId: html_id.clone(),
        body: html,
    };
    if let Err(error) = tx.send(EntryToSend::Entry(entry)) {
        error!("Broadcasting failed {}", error);
    }
    Ok(())
}
