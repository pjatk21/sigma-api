use std::{error::Error, fmt::Display};

use chrono::{DateTime, Utc};
use kuchiki::NodeRef;
use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct TimeTableEntry {
    pub(crate) title: Option<String>,
    pub(crate) persons: Vec<String>,
    pub(crate) details: Option<String>,
    pub(crate) type_of: String,
    pub(crate) subjects: Vec<String>,
    pub(crate) subject_codes: Vec<String>,
    pub(crate) groups: Vec<String>,
    pub(crate) students_count: Option<String>,
    pub(crate) building: String,
    pub(crate) room: String,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub(crate) datetime_beginning: DateTime<Utc>,
    #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
    pub(crate) datetime_ending: DateTime<Utc>,
}

impl TryFrom<NodeRef> for TimeTableEntry {
    type Error = Box<dyn Error>;

    fn try_from(dom: NodeRef) -> Result<Self, Self::Error> {
        let date = get_data(&dom, "#ctl06_DataZajecLabel", "date")?;
        let hour_beginning = get_data(&dom, "#ctl06_GodzRozpLabel", "hour_beginning")?;
        let hour_ending = get_data(&dom, "#ctl06_GodzZakonLabel", "hour_ending")?;
        let beginning_str = format!("{} {} +0100", date, hour_beginning);
        let ending_str = format!("{} {} +0100", date, hour_ending);
        let datetime_beginning =
            DateTime::parse_from_str(&beginning_str, "%d.%m.%Y %T %z")?.with_timezone(&Utc);
        let datetime_ending =
            DateTime::parse_from_str(&ending_str, "%d.%m.%Y %T %z")?.with_timezone(&Utc);
        let result = TimeTableEntry {
            title: get_data_option(&dom, "#ctl06_TytulRezerwacjiLabel"),
            persons: get_multiple_data(
                &dom,
                "#ctl06_OsobaRezerwujacaLabel, #ctl06_DydaktycyLabel",
                "persons",
            )?,
            details: get_data_option(&dom,"#ctl06_OpisLabel"),
            type_of: get_data(
                &dom,
                "#ctl06_TypRezerwacjiLabel, #ctl06_TypZajecLabel",
                "type_of",
            )?,
            subjects: get_multiple_data(
                &dom,
                "#ctl06_NazwyPrzedmiotowLabel, #ctl06_NazwaPrzedmiotyLabel",
                "subjects",
            )?,
            subject_codes: get_multiple_data(
                &dom,
                "#ctl06_KodyPrzedmiotowLabel, #ctl06_KodPrzedmiotuLabel",
                "subject_codes",
            )?,
            groups: get_multiple_data(
                &dom,
                "#ctl06_GrupyStudenckieLabel, #ctl06_GrupyLabel",
                "groups",
            )?,
            students_count: get_data_option(&dom, "#ctl06_LiczbaStudentowLabel"),
            building: get_data(&dom, "#ctl06_BudynekLabel", "building")?,
            room: get_data(&dom, "#ctl06_SalaLabel", "room")?,
            datetime_beginning,
            datetime_ending,
        };
        Ok(result)
    }
}

fn get_data_option(dom: &NodeRef, selector: &'static str) -> Option<String> {
    if let Ok(dom) = dom.select_first(selector) {
        Some(dom.text_contents().trim().to_string())
    } else {
        None
    }
}

fn get_data(
    dom: &NodeRef,
    selector: &'static str,
    cause: &'static str,
) -> Result<String, ParseError> {
    let date = dom
        .select_first(selector)
        .map_err(|_| ParseError { cause })?
        .text_contents()
        .trim()
        .to_string();
    Ok(date)
}

fn get_multiple_data(
    dom: &NodeRef,
    selectors: &'static str,
    cause: &'static str,
) -> Result<Vec<String>, ParseError> {
    Ok(dom
        .select_first(selectors)
        .map_err(|_| ParseError { cause })?
        .text_contents()
        .trim()
        .split_terminator(',')
        .into_iter()
        .map(|a| a.trim().to_string())
        .collect())
}

#[derive(Debug)]
struct ParseError {
    cause: &'static str,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::write(f, format_args!("Parse error: {}", self.cause))
    }
}

impl Error for ParseError {
    fn cause(&self) -> Option<&dyn Error> {
        None
    }
}
