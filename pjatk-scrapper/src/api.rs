#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HypervisorRequest {
    scrapper: String,
    command: HypervisorCommand,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum HypervisorCommand {
    Disconnect,
    Exit,
    Scrap,
    Queue,
    Cancel,
}
