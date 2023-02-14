#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use std::{error::Error, time::Duration};

use kuchiki::traits::TendrilSink;
use thirtyfour::{
    prelude::{ElementQueryable, ElementWaitable},
    By, Key, WebDriver,
};
use timetable::timetable::TimeTableEntry;
use tokio::sync::mpsc::UnboundedSender;
use tracing::info;

#[derive(Debug)]
pub(crate) enum EntryToSend {
    Entry(Box<TimeTableEntry>),
    Quit,
}

pub(crate) async fn parse_timetable_day(
    web_driver: &WebDriver,
    date: String,
    tx: UnboundedSender<EntryToSend>,
) -> Result<(), Box<dyn Error>> {
    let date_input = web_driver.find(By::Id("DataPicker_dateInput")).await?;

    date_input.click().await?;
    date_input.send_keys(Key::Control + "a").await?;
    date_input.send_keys(date.clone() + Key::Enter).await?;

    tokio::time::sleep(Duration::from_secs(1)).await;

    let table = web_driver.find(By::Id("ZajeciaTable")).await?;
    let good_elements = table.find_all(By::Css("tbody td[id*=\";\"]")).await?;

    let count = good_elements.len();
    info!("Found {} timetable entries", count);
    let window_rect = web_driver.get_window_rect().await?;
    for (index, element) in good_elements.iter().enumerate() {
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
        let tooltip_node = kuchiki::parse_html().from_utf8().one(html.as_bytes());
        let entry: TimeTableEntry = tooltip_node.try_into()?;
        tx.send(EntryToSend::Entry(Box::new(entry)))?;
        info!("{}", index);
    }
    Ok(())
}
