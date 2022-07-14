#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use super::base_validation::BaseValidation;
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct EntryRequest<T: AsRef<str>> {
    RadScriptManager1: &'static str,
    __EVENTTARGET: &'static str,
    __EVENTARGUMENT: &'static str,
    RadToolTipManager1_ClientState: String,
    #[serde(flatten)]
    base_validation: BaseValidation<T>,
}

impl<T: AsRef<str>> EntryRequest<T> {
    pub(crate) fn new(html_id: String, base_validation: BaseValidation<T>) -> Self {
        let html_id_client_state = format!(
            "{{\"AjaxTargetControl\":\"{0}\",\"Value\":\"{0}\"}}",
            &html_id
        );
        Self {
            RadScriptManager1: "RadToolTipManager1RTMPanel|RadToolTipManager1RTMPanel",
            __EVENTTARGET: "RadToolTipManager1RTMPanel",
            __EVENTARGUMENT: "undefined",
            RadToolTipManager1_ClientState: html_id_client_state,
            base_validation,
        }
    }
}
