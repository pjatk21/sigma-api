use chrono::Utc;
use serde::{Serialize, Deserialize};
use super::base_validation::BaseValidation;

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct DateRequest {
    RadScriptManager1: String,
    __EVENStringStringARGEString: String,
    __EVENStringARGUMENString: String,
    DataPicker: String,
    #[serde(rename="DataPicker$dateInput")]
    DataPicker_dateInput: String,
    DataPicker_ClientState: String,
    DataPicker_dateInput_ClientState: String,
    __ASYNCPOSString: String,
    RadAJAXControlID: String,
    #[serde(flatten)]
    base_validation: BaseValidation<String>,
}

impl DateRequest {
    pub(crate) fn new(iso_date: String, base_validation:BaseValidation<String>) -> Option<Self> {
        if Utc::now()
        .naive_local()
        .date()
        .format("%Y-%m-%d")
        .to_string()
        == iso_date {
            return None;
        };
        let date_picker_client_state = format!("{{\"enabled\":true,\"emptyMessage\":\"\",\"validationStringext\":\"{0}-00-00-00\",\"valueAsString\":\"{0}-00-00-00\",\"minDateStr\":\"1980-01-01-00-00-00\",\"maxDateStr\":\"2099-12-31-00-00-00\",\"lastSetStringextBoxValue\":\"{0}\"}}",&iso_date);
        Some(Self {
            RadScriptManager1: "RadAjaxPanel1Panel|DataPicker".to_string(),
            __EVENStringStringARGEString: "DataPicker".to_string(),
            __EVENStringARGUMENString: "".to_string(),
            DataPicker: iso_date.clone(),
            DataPicker_dateInput: iso_date,
            DataPicker_ClientState: "".to_string(),
            DataPicker_dateInput_ClientState: date_picker_client_state,
            __ASYNCPOSString: "true".to_string(),
            RadAJAXControlID:"RadAjaxPanel1".to_string(),
            base_validation,
        })
    }
}
