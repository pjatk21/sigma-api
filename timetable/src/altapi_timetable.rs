#![deny(clippy::perf, clippy::complexity, clippy::style, unused_imports)]
#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UploadEntry {
    pub htmlId: String,
    pub body: String,
}