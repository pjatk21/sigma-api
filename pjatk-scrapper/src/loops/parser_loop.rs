#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use reqwest::IntoUrl;
use std::{error::Error, time::Duration};

use crate::{
    request::base_validation::BaseValidation,
    scraper::{parse_timetable_day, EntryToSend},
};
use chrono::DateTime;
use crossbeam::utils::Backoff;
use kuchiki::traits::TendrilSink;
use reqwest::{header::*, Client};
use tokio::sync::broadcast::{error::RecvError, Receiver, Sender};
use tracing::{info, info_span, trace, warn};

pub(crate) struct ParserLoop<'a, T: AsRef<str>> {
    tx: Sender<EntryToSend>,
    rx: Receiver<EntryToSend>,
    client: &'a reqwest::Client,
    base_validation: BaseValidation<String>,
    url: T,
    timeout: Duration,
    instant: tokio::time::Instant,
    max_concurrent: usize,
}

impl<'a, T: AsRef<str>> ParserLoop<'a, T> {
    pub(crate) async fn new(
        tx: Sender<EntryToSend>,
        client: &'a reqwest::Client,
        url: T,
        timeout: Duration,
        max_concurrent: usize,
    ) -> Result<ParserLoop<'a, T>, Box<dyn Error>> {
        let instant = tokio::time::Instant::now();
        let (base_validation, _) =
            ParserLoop::<&str>::get_base_validation_and_html(url.as_ref().to_string(), client)
                .await?;
        info!("Init: {} sec.", instant.elapsed().as_secs());
        let rx = tx.subscribe();
        Ok(Self {
            tx,
            client,
            base_validation,
            url,
            timeout,
            instant: tokio::time::Instant::now(),
            max_concurrent,
            rx,
        })
    }
    pub(crate) fn get_base_headers() -> Result<HeaderMap, Box<dyn Error>> {
        let mut base_headers = HeaderMap::new();
        base_headers.append(
            USER_AGENT,
            "Mozilla/5.0 (X11; Fedora; Linux x86_64; rv:98.0) Gecko/20100101 Firefox/98.0"
                .parse()?,
        );
        base_headers.append(
            CONTENT_TYPE,
            "application/x-www-form-urlencoded; charset=utf-8".parse()?,
        );
        base_headers.append("X-MicrosoftAjax", "Delta=true".parse()?);
        Ok(base_headers)
    }

    pub(crate) async fn start(&mut self) {
        loop {
            let entry_result = self.rx.recv().await;
            trace!("Message received");
            let backoff = Backoff::new();
            match entry_result {
                Ok(entry) => match entry {
                    EntryToSend::HypervisorCommand(hypervisor_command) => {
                        let span = info_span!("Parsing entries");
                        let date = DateTime::parse_from_rfc3339(&hypervisor_command.scrapUntil)
                            .expect("Bad DateTime format until!");
                        let date_str = date.format("%Y-%m-%d").to_string();
                        self.instant = tokio::time::Instant::now();
                        let number = parse_timetable_day(
                            self.client,
                            date_str,
                            self.tx.clone(),
                            &mut self.base_validation,
                            self.url.as_ref().to_string(),
                            self.max_concurrent,
                        )
                        .await
                        .expect("Parsing failed!");
                        let duration = &self.instant.elapsed().as_secs_f32();
                        let rate = if number != 0 {
                            number as f32 / duration
                        } else {
                            0.
                        };
                        span.in_scope(|| {
                            info!(
                                "Scrapping ended!: {} in ~{} sec. ({} req/s)",
                                date,
                                &self.instant.elapsed().as_secs(),
                                rate
                            );
                        });
                        self.tx
                            .send(EntryToSend::HypervisorFinish("finished"))
                            .expect("`finish`-ing failed!");
                        info!("Sleeping for {} miliseconds...", self.timeout.as_millis());
                        tokio::time::sleep(self.timeout).await;
                    }
                    EntryToSend::Quit => {
                        let span = info_span!("Parsing entries");
                        span.in_scope(|| {
                            info!("Closing scraper thread!");
                        });
                        break;
                    }
                    _ => {
                        backoff.snooze();
                    }
                },
                Err(recv_error) => match recv_error {
                    RecvError::Lagged(number) => {
                        warn!("Receiver overflow! Skipping: {}", number);
                        backoff.snooze();
                    }
                    RecvError::Closed => {
                        break;
                    }
                },
            }
            tokio::time::sleep(Duration::from_nanos(250)).await;
        }
    }

    pub(crate) async fn get_base_validation_and_html<R>(
        url: R,
        client: &Client,
    ) -> Result<(BaseValidation<String>, String), Box<dyn Error>>
    where
        R: AsRef<str> + IntoUrl,
    {
        let response = client
            .get(url)
            .headers(ParserLoop::<String>::get_base_headers()?)
            .send()
            .await?;
        let bytes = response.bytes().await?;
        let html_string = std::str::from_utf8(bytes.as_ref())?;
        let mut temp = BaseValidation::new("".to_string(), "".to_string(), "".to_string());
        ParserLoop::<&str>::update_base_validation_and_give_html_full(html_string, &mut temp)
            .await
            .expect("Updating failed!");
        Ok((temp, html_string.to_string()))
    }

    pub(crate) async fn update_base_validation_and_give_html_full(
        html_string: T,
        base_validation: &mut BaseValidation<String>,
    ) -> Result<T, ()> {
        let node_ref = kuchiki::parse_html().one(html_string.as_ref());

        let view_state_dom = node_ref.select_first("#__VIEWSTATE").unwrap();
        let view_state_attributes = view_state_dom.attributes.borrow();
        let view_state = view_state_attributes.get("value").unwrap();

        let view_state_generator_dom = node_ref.select_first("#__VIEWSTATEGENERATOR").unwrap();
        let view_state_generator_attributes = view_state_generator_dom.attributes.borrow();
        let view_state_generator = view_state_generator_attributes.get("value").unwrap();

        let event_validation_dom = node_ref.select_first("#__EVENTVALIDATION").unwrap();
        let event_validation_attributes = event_validation_dom.attributes.borrow();
        let event_validation = event_validation_attributes.get("value").unwrap();

        base_validation.update(
            view_state.to_string(),
            view_state_generator.to_string(),
            event_validation.to_string(),
        );
        Ok(html_string)
    }

    pub(crate) async fn give_html_delta(html: T, type_of: T) -> String {
        let splitted = html.as_ref().split('|');

        let position_html = splitted.clone().position(|x| x == type_of.as_ref());

        let splitted_vec: Vec<&str> = splitted.collect();

        splitted_vec[position_html.unwrap() + 1].to_string()
    }
}
