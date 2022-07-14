#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use super::base_validation::BaseValidation;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct DateRequest<T: AsRef<str> + Clone> {
    RadScriptManager1: &'static str,
    __EVENTTARGET: &'static str,
    __EVENTARGUMENT: &'static str,
    DataPicker: T,
    #[serde(rename = "DataPicker$dateInput")]
    DataPicker_dateInput: T,
    DataPicker_ClientState: &'static str,
    DataPicker_dateInput_ClientState: String,
    __ASYNCPOST: &'static str,
    RadAJAXControlID: &'static str,
    #[serde(flatten)]
    base_validation: BaseValidation<T>,
}

impl<T: AsRef<str> + Clone> DateRequest<T> {
    pub(crate) fn new(iso_date: T, base_validation: BaseValidation<T>) -> Option<Self> {
        if Utc::now()
            .naive_local()
            .date()
            .format("%Y-%m-%d")
            .to_string()
            == iso_date.as_ref()
        {
            return None;
        };
        let date_picker_client_state = format!("{{\"enabled\":true,\"emptyMessage\":\"\",\"validationText\":\"{0}-00-00-00\",\"valueAsString\":\"{0}-00-00-00\",\"minDateStr\":\"1980-01-01-00-00-00\",\"maxDateStr\":\"2099-12-31-00-00-00\",\"lastSetTextBoxValue\":\"{0}\"}}", iso_date.as_ref());
        Some(Self {
            RadScriptManager1: "RadAjaxPanel1Panel|DataPicker",
            __EVENTTARGET: "DataPicker",
            __EVENTARGUMENT: "",
            DataPicker: iso_date.clone(),
            DataPicker_dateInput: iso_date,
            DataPicker_ClientState: "",
            DataPicker_dateInput_ClientState: date_picker_client_state,
            __ASYNCPOST: "true",
            RadAJAXControlID: "RadAjaxPanel1",
            base_validation,
        })
    }
}
