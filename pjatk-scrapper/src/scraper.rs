#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::error::Error;
use std::time::Duration;

use thirtyfour::error::WebDriverError;

use thirtyfour::{
    prelude::{ElementQueryable, ElementWaitable},
    By, Keys, WebDriver, WebElement, Rect,
};
use timetable::altapi_timetable::UploadEntry;
use tokio::sync::broadcast::Sender;

use tracing::{info, warn, trace, error};

use crate::api::HypervisorCommand;

#[derive(Debug, Clone)]
pub enum EntryToSend {
    HypervisorCommand(HypervisorCommand),
    Entry(Box<UploadEntry>),
    HypervisorFinish(&'static str),
    Quit,
}

#[tracing::instrument]
pub(crate) async fn parse_timetable_day(
    web_driver: &WebDriver,
    date: String,
    tx: Sender<EntryToSend>,
) -> Result<(), Box<dyn Error>> {
    let date_input = web_driver
        .find_element(By::Id("DataPicker_dateInput"))
        .await?;
    let text = date_input.get_property("value").await?.unwrap_or_else(|| "".to_string());
    trace!("Found date: {}",text);
    if date != text {
        date_input.click().await?;
        date_input.send_keys(Keys::Control + "a").await?;
        date_input.send_keys(date.clone()).await?;
        date_input.send_keys(Keys::Enter).await?;
    } else {
        warn!("Can't update timetable, when given date is representing this day!");
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
    

    let table = web_driver.find_element(By::Id("ZajeciaTable")).await?;
    let good_elements = table.find_elements(By::Css("tbody td[id*=\";\"]")).await?;

    let count = good_elements.len();
    info!("Found {} timetable entries", count);

    let mut faulty_elements_tier_1: Vec<String> = vec![];
    let mut faulty_elements_tier_2: Vec<String> = vec![];

    let window_rect = web_driver.get_window_rect().await?;

    // Normal scrapping (5-sec. timeout)
    for (index, element) in good_elements.iter().enumerate() {
        let htmlId = element.id().await?.unwrap();
        parse_timetable_entry(htmlId,web_driver,window_rect.clone(),date.clone(),tx.clone(),Some(&mut faulty_elements_tier_1),5).await?;
        info!("{}", index);
        tokio::time::sleep(Duration::from_nanos(250)).await;
    }
    // Tier-1 failure scrapping (10-sec. timeout)
    for (index, htmlId) in faulty_elements_tier_1.into_iter().enumerate() {
        parse_timetable_entry(htmlId,web_driver,window_rect.clone(),date.clone(),tx.clone(),Some(&mut faulty_elements_tier_2),10).await?;
        info!("{}", index);
        tokio::time::sleep(Duration::from_nanos(250)).await;
    }
    // Tier-2 failure scrapping (30-sec timeout)
    for (index, htmlId) in faulty_elements_tier_2.into_iter().enumerate() {
        parse_timetable_entry(htmlId,web_driver,window_rect.clone(),date.clone(),tx.clone(),None,30).await?;
        info!("{}", index);
        tokio::time::sleep(Duration::from_nanos(250)).await;
    }

    Ok(())
}

#[tracing::instrument]
pub(crate) async fn parse_timetable_entry(
    htmlId: String,
    web_driver: &WebDriver,
    window_rect: Rect,
    date: String,
    tx: Sender<EntryToSend>,
    faulty_elements: Option<&mut Vec<String>>,
    timeout: u64
) -> Result<(),Box<WebDriverError>> {
    let element = web_driver.find_element(By::Id(&htmlId)).await?;
    let (x, y) = element.rect().await?.icenter();
        if x > window_rect.x || y > window_rect.y || x < 0 || y < 0 {
            element.scroll_into_view().await?;
        }
        element.wait_until().clickable().await?;
        if let Err(err) = web_driver
            .action_chain()
            .send_keys(Keys::Escape)
            .move_to_element_center(&element)
            .perform()
            .await
        {
            return Err(Box::new(err));
        }
        let tooltip_element: WebElement = match web_driver
            .query(By::Id("RadToolTipManager1RTMPanel"))
            .wait(Duration::from_secs(timeout), Duration::from_nanos(125))
            .and_displayed()
            .first()
            .await {
                Ok(element) => element,
                Err(_) => {
                    if let Some(vec) = faulty_elements {
                        warn!("Tooltip timeout exceeded {} sec! Trying again at the end... ({} at {})", &timeout, &date, &htmlId);
                        vec.push(htmlId);
                        return Ok(());
                    } else {
                        error!("Tooltip timeout again... ({} at {})",&date, &htmlId);
                        return Ok(());
                    }
                }
            };
        let html = tooltip_element.inner_html().await?;
        let entry: UploadEntry = UploadEntry {
            htmlId: htmlId.clone(),
            body: html,
        };
        if let Err(error) = tx.send(EntryToSend::Entry(Box::new(entry))) {
            if let Some(vec) = faulty_elements {
                warn!("Broadcasting failed, trying again... {}", error);
                vec.push(htmlId.clone());
            } else {
                error!("Broadcasting failed again... {}", error);
            }
        }
        Ok(())
}