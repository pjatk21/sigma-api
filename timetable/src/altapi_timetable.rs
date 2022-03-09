#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadEntry {
    pub html_id: String,
    pub body: String,
}