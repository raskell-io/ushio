//! HAR (HTTP Archive) parsing
//!
//! Parses HAR 1.2 format files into ushio's internal capture format.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// HAR 1.2 root structure
#[derive(Debug, Deserialize)]
pub struct Har {
    pub log: HarLog,
}

#[derive(Debug, Deserialize)]
pub struct HarLog {
    pub version: String,
    pub creator: HarCreator,
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize)]
pub struct HarCreator {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarEntry {
    pub started_date_time: String,
    pub request: HarRequest,
    pub response: HarResponse,
    pub time: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarRequest {
    pub method: String,
    pub url: String,
    pub http_version: String,
    pub headers: Vec<HarHeader>,
    pub query_string: Vec<HarQueryParam>,
    pub post_data: Option<HarPostData>,
}

#[derive(Debug, Deserialize)]
pub struct HarResponse {
    pub status: u16,
    #[serde(rename = "statusText")]
    pub status_text: String,
    pub headers: Vec<HarHeader>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HarHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct HarQueryParam {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HarPostData {
    pub mime_type: String,
    pub text: Option<String>,
}

/// Parse a HAR file from JSON string
pub fn parse_har(content: &str) -> Result<Har> {
    let har: Har = serde_json::from_str(content)?;
    Ok(har)
}

/// Convert HAR entries to ushio capture format
pub fn har_to_capture(har: Har) -> Vec<crate::capture::CapturedRequest> {
    har.log
        .entries
        .into_iter()
        .map(|entry| crate::capture::CapturedRequest {
            method: entry.request.method,
            url: entry.request.url,
            headers: entry
                .request
                .headers
                .into_iter()
                .map(|h| (h.name, h.value))
                .collect(),
            body: entry.request.post_data.and_then(|p| p.text),
            expected_status: Some(entry.response.status),
        })
        .collect()
}
