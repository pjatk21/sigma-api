#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use futures::StreamExt;
use regex::Regex;
use reqwest::IntoUrl;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Debug, Display};

use timetable::altapi_timetable::UploadEntry;
use tokio::sync::broadcast::Sender;

use tracing::{error, info};

use crate::api::HypervisorCommand;
use crate::loops::parser_loop::ParserLoop;
use crate::request::base_validation::BaseValidation;
use crate::request::date_request::DateRequest;
use crate::request::entry_request::EntryRequest;
use lazy_static::lazy_static;

#[derive(Debug, Clone)]
pub enum EntryToSend {
    HypervisorCommand(HypervisorCommand),
    Entry(UploadEntry),
    HypervisorFinish(&'static str),
    Quit,
}

#[tracing::instrument(skip(base_validation,tx,http_client))]
pub(crate) async fn parse_timetable_day(
    http_client: &reqwest::Client,
    date: String,
    tx: Sender<EntryToSend>,
    base_validation: &mut BaseValidation<String>,
    url: String,
    max_concurrent: usize,
) -> Result<usize, Box<dyn Error>> {
    let date_form =
        DateRequest::new(date.clone(),base_validation.clone());
    let response = http_client
        .post(url.clone())
        .form(&date_form)
        .send()
        .await?;

    let bytes = response.bytes().await?;
    let html_body = std::str::from_utf8(bytes.as_ref())?;
    let html_string: String = if date_form.is_some() {
        ParserLoop::<&str>::give_html_delta(html_body, "RadAjaxPanel1Panel")
            .await
    } else {
        ParserLoop::<&str>::update_base_validation_and_give_html_full(
            html_body,
            base_validation,
        )
        .await;
        html_body.to_string()
    };
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
    let date_ref = date.as_str();
    let url_ref = url.as_str();
    let tx_ref = &tx;
    let base_validation_ref = &base_validation;

    // Normal scrapping (5-sec. timeout)

    futures::stream::iter(good_elements).for_each_concurrent(max_concurrent,|html_id| async move {
        parse_timetable_entry(
            html_id,
            http_client,
            date_ref,
            tx_ref,
            5,
            base_validation_ref,
            url_ref,
        )
        .await
        .unwrap_or_else(|err| panic!("Failed at: {}\n{:#?}", html_id, err));
    }).await;

    
    Ok(count)
}

#[tracing::instrument(skip(http_client,tx,base_validation))]
pub(crate) async fn parse_timetable_entry<T,R>(
    html_id: R,
    http_client: &reqwest::Client,
    date: T,
    tx: &Sender<EntryToSend>,
    timeout: u64,
    base_validation: &BaseValidation<String>,
    url: T,
) -> Result<(), Box<dyn Error>>
where
    T: AsRef<str> + Debug + Display + IntoUrl,
    R: AsRef<str> + IntoUrl + Debug + Copy
{
    let timetable_entry_form = EntryRequest::new(html_id.as_ref().to_string(), base_validation.clone());
    let response = http_client
        .post(url)
        .form(&timetable_entry_form)
        .send()
        .await?;
    let response_bytes = response.bytes().await?;
    let response_string = std::str::from_utf8(response_bytes.as_ref())?;

    let html = ParserLoop::<&str>::give_html_delta(
        response_string,
        "RadToolTipManager1RTMPanel"
    )
    .await;

    let entry: UploadEntry = UploadEntry {
        htmlId: html_id.as_ref().to_string(),
        body: html,
    };
    if let Err(error) = tx.send(EntryToSend::Entry(entry)) {
        error!("Broadcasting failed {}", error);
    }
    Ok(())
}
