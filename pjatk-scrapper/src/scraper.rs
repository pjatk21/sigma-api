#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::{error::Error, time::Duration};

use thirtyfour::{
    prelude::{ElementQueryable, ElementWaitable},
    By, Keys, WebDriver,
};
use timetable::altapi_timetable::UploadEntry;
use async_broadcast::{Sender, TrySendError};

use tracing::{info, warn, trace};

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
    async_std::task::sleep(Duration::from_secs(1)).await;
    

    let table = web_driver.find_element(By::Id("ZajeciaTable")).await?;
    let good_elements = table.find_elements(By::Css("tbody td[id*=\";\"]")).await?;

    let count = good_elements.len();
    info!("Found {} timetable entries", count);
    let window_rect = web_driver.get_window_rect().await?;
    for (index, element) in good_elements.iter().enumerate() {
        let htmlId = element.id().await?.unwrap();
        let (x, y) = element.rect().await?.icenter();
        if x > window_rect.x || y > window_rect.y || x < 0 || y < 0 {
            element.scroll_into_view().await?;
        }
        element.wait_until().clickable().await?;
        if let Err(err) = web_driver
            .action_chain()
            .move_to_element_center(element)
            .click()
            .perform()
            .await
        {
            eprintln!("Unexpected error: {:#?}", err);
            break;
        }
        let tooltip_element = web_driver
            .query(By::Id("RadToolTipManager1RTMPanel"))
            .wait(Duration::MAX, Duration::from_nanos(125))
            .and_displayed()
            .first()
            .await?;
        let html = tooltip_element.inner_html().await?;
        let entry: UploadEntry = UploadEntry {
            htmlId,
            body: html,
        };
        while let Err(TrySendError::Full(_)) = 
        tx.try_broadcast(EntryToSend::Entry(Box::new(entry.clone()))) {
            warn!("Broadcast failed!, trying again!");
            async_std::task::sleep(Duration::from_nanos(250)).await;

        }
        info!("{}", index);
    }
    Ok(())
}
