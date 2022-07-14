#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
use serde::{Deserialize, Serialize};

#[allow(non_snake_case)]
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct BaseValidation<T: AsRef<str>> {
    __VIEWSTATE: T,
    __VIEWSTATEGENERATOR: T,
    __EVENTVALIDATION: T,
}

impl<T: AsRef<str>> BaseValidation<T> {
    pub(crate) fn new(view_state: T, view_state_generator: T, event_validation: T) -> Self {
        Self {
            __VIEWSTATE: view_state,
            __VIEWSTATEGENERATOR: view_state_generator,
            __EVENTVALIDATION: event_validation,
        }
    }
    pub(crate) fn update(&mut self, view_state: T, view_state_generator: T, event_validation: T) {
        self.__VIEWSTATE = view_state;
        self.__EVENTVALIDATION = event_validation;
        self.__VIEWSTATEGENERATOR = view_state_generator;
    }
}
