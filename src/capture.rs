//! Capture format and storage
//!
//! Ushio's internal format for representing captured HTTP traffic.

use serde::{Deserialize, Serialize};

/// A captured HTTP request for replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
    pub expected_status: Option<u16>,
}

/// A capture file containing multiple requests
#[derive(Debug, Serialize, Deserialize)]
pub struct Capture {
    pub version: String,
    pub source: Option<String>,
    pub requests: Vec<CapturedRequest>,
}

impl Capture {
    pub fn new(requests: Vec<CapturedRequest>) -> Self {
        Self {
            version: "1.0".to_string(),
            source: None,
            requests,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }
}

/// Load a capture from a file
pub fn load_capture(path: &str) -> anyhow::Result<Capture> {
    let content = std::fs::read_to_string(path)?;
    let capture: Capture = serde_json::from_str(&content)?;
    Ok(capture)
}

/// Save a capture to a file
pub fn save_capture(capture: &Capture, path: &str) -> anyhow::Result<()> {
    let content = serde_json::to_string_pretty(capture)?;
    std::fs::write(path, content)?;
    Ok(())
}
