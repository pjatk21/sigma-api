#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]

use poem_openapi::Object;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Object)]
pub struct UploadEntry {
    pub html_id: String,
    pub body: String,
}