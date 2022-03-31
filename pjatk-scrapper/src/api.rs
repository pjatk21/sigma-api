#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HypervisorCommand {
    pub scrapStart: Option<String>,
    pub scrapUntil: String,
    pub limit: Option<usize>,
    pub skip: Option<usize>,
}
