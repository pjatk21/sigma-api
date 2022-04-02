use super::base_validation::BaseValidation;
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct EntryRequest {
    RadScriptManager1: String,
    __EVENTTARGET: String,
    __EVENTARGUMENT: String,
    RadToolTipManager1_ClientState: String,
    #[serde(flatten)]
    base_validation: BaseValidation<String>,
}

impl EntryRequest {
    pub(crate) fn new(html_id: String,base_validation: BaseValidation<String>) -> Self {
        let html_id_client_state = format!(
            "{{\"AjaxTargetControl\":\"{0}\",\"Value\":\"{0}\"}}",
            &html_id
        );
        Self {
            RadScriptManager1: "RadToolTipManager1RTMPanel|RadToolTipManager1RTMPanel".to_string(),
            __EVENTTARGET: "RadToolTipManager1RTMPanel".to_string(),
            __EVENTARGUMENT: "undefined".to_string(),
            RadToolTipManager1_ClientState: html_id_client_state,
            base_validation,
        }
    }
}
