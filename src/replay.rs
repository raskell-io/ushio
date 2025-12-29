//! Traffic replay engine
//!
//! Replays captured requests against target endpoints in deterministic order.

use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use url::Url;

use crate::capture::CapturedRequest;

/// Configuration for replay execution
#[derive(Debug, Clone)]
pub struct ReplayConfig {
    pub timeout: Duration,
    pub concurrency: usize,
    pub header_mutations: Vec<(String, String)>,
    pub strip_cookies: bool,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            concurrency: 1,
            header_mutations: vec![],
            strip_cookies: false,
        }
    }
}

/// Result of replaying a single request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub request_index: usize,
    pub method: String,
    pub url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body_size: usize,
    pub duration_ms: u64,
    pub expected_status: Option<u16>,
    pub status_match: bool,
    pub error: Option<String>,
}

/// Result of a complete replay session
#[derive(Debug, Serialize, Deserialize)]
pub struct ReplaySession {
    pub target: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub total_requests: usize,
    pub successful: usize,
    pub failed: usize,
    pub status_mismatches: usize,
    pub results: Vec<ReplayResult>,
}

/// Replay a set of requests against a target
pub async fn replay(
    requests: &[CapturedRequest],
    target: &str,
    config: ReplayConfig,
) -> Result<ReplaySession> {
    let target_url = Url::parse(target).context("Invalid target URL")?;

    // Build HTTP client
    let client = reqwest::Client::builder()
        .timeout(config.timeout)
        .redirect(reqwest::redirect::Policy::none()) // Don't follow redirects
        .build()
        .context("Failed to build HTTP client")?;

    let mut results = Vec::with_capacity(requests.len());
    let mut successful = 0;
    let mut failed = 0;
    let mut status_mismatches = 0;

    // Process requests sequentially for determinism
    for (index, request) in requests.iter().enumerate() {
        let result = replay_single(&client, request, index, &target_url, &config).await;

        match &result {
            Ok(r) => {
                if r.error.is_some() {
                    failed += 1;
                } else {
                    successful += 1;
                    if !r.status_match {
                        status_mismatches += 1;
                    }
                }
            }
            Err(_) => {
                failed += 1;
            }
        }

        results.push(result.unwrap_or_else(|e| ReplayResult {
            request_index: index,
            method: request.method.clone(),
            url: rewrite_url(&request.url, &target_url).unwrap_or_else(|_| request.url.clone()),
            status: 0,
            headers: vec![],
            body_size: 0,
            duration_ms: 0,
            expected_status: request.expected_status,
            status_match: false,
            error: Some(e.to_string()),
        }));
    }

    Ok(ReplaySession {
        target: target.to_string(),
        timestamp: chrono::Utc::now(),
        total_requests: requests.len(),
        successful,
        failed,
        status_mismatches,
        results,
    })
}

/// Replay a single request
async fn replay_single(
    client: &reqwest::Client,
    request: &CapturedRequest,
    index: usize,
    target_url: &Url,
    config: &ReplayConfig,
) -> Result<ReplayResult> {
    // Rewrite URL to target
    let url = rewrite_url(&request.url, target_url)?;

    // Build headers
    let headers = apply_mutations(&request.headers, &config.header_mutations, config.strip_cookies);
    let header_map = build_header_map(&headers)?;

    // Build request
    let method: reqwest::Method = request.method.parse().context("Invalid HTTP method")?;
    let mut req = client.request(method, &url).headers(header_map);

    // Add body if present
    if let Some(ref body) = request.body {
        req = req.body(body.clone());
    }

    // Execute with timing
    let start = Instant::now();
    let response = req.send().await.context("Request failed")?;
    let duration = start.elapsed();

    let status = response.status().as_u16();
    let response_headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = response.bytes().await.context("Failed to read response body")?;
    let body_size = body.len();

    let status_match = request
        .expected_status
        .map(|expected| expected == status)
        .unwrap_or(true);

    Ok(ReplayResult {
        request_index: index,
        method: request.method.clone(),
        url,
        status,
        headers: response_headers,
        body_size,
        duration_ms: duration.as_millis() as u64,
        expected_status: request.expected_status,
        status_match,
        error: None,
    })
}

/// Rewrite a URL to use the target host
fn rewrite_url(original: &str, target: &Url) -> Result<String> {
    let mut url = Url::parse(original).context("Invalid original URL")?;

    // Replace scheme, host, and port with target
    url.set_scheme(target.scheme()).ok();
    url.set_host(target.host_str()).ok();
    url.set_port(target.port()).ok();

    Ok(url.to_string())
}

/// Apply header mutations to a request
fn apply_mutations(
    headers: &[(String, String)],
    mutations: &[(String, String)],
    strip_cookies: bool,
) -> Vec<(String, String)> {
    let mut result: Vec<(String, String)> = headers
        .iter()
        .filter(|(name, _)| {
            let name_lower = name.to_lowercase();
            // Skip cookies if configured
            if strip_cookies && name_lower == "cookie" {
                return false;
            }
            // Skip host header (will be set by reqwest)
            if name_lower == "host" {
                return false;
            }
            // Skip content-length (will be set by reqwest)
            if name_lower == "content-length" {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    // Apply mutations
    for (name, value) in mutations {
        if value.is_empty() {
            // Remove header
            result.retain(|(n, _)| !n.eq_ignore_ascii_case(name));
        } else {
            // Add or replace header
            let pos = result.iter().position(|(n, _)| n.eq_ignore_ascii_case(name));
            if let Some(idx) = pos {
                result[idx] = (name.clone(), value.clone());
            } else {
                result.push((name.clone(), value.clone()));
            }
        }
    }

    result
}

/// Build a HeaderMap from header tuples
fn build_header_map(headers: &[(String, String)]) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();

    for (name, value) in headers {
        let header_name: HeaderName = name.parse().context(format!("Invalid header name: {}", name))?;
        let header_value: HeaderValue = value
            .parse()
            .context(format!("Invalid header value for {}", name))?;
        map.insert(header_name, header_value);
    }

    Ok(map)
}

/// Save a replay session to a file
pub fn save_session(session: &ReplaySession, path: &str) -> Result<()> {
    let content = serde_json::to_string_pretty(session)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Load a replay session from a file
pub fn load_session(path: &str) -> Result<ReplaySession> {
    let content = std::fs::read_to_string(path)?;
    let session: ReplaySession = serde_json::from_str(&content)?;
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rewrite_url() {
        let target = Url::parse("https://staging.example.com").unwrap();
        let result = rewrite_url("https://prod.example.com/api/users?q=test", &target).unwrap();
        assert_eq!(result, "https://staging.example.com/api/users?q=test");
    }

    #[test]
    fn test_rewrite_url_with_port() {
        let target = Url::parse("https://staging.example.com:8443").unwrap();
        let result = rewrite_url("https://prod.example.com/api/users", &target).unwrap();
        assert_eq!(result, "https://staging.example.com:8443/api/users");
    }

    #[test]
    fn test_apply_mutations_add() {
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let mutations = vec![("Authorization".to_string(), "Bearer token".to_string())];
        let result = apply_mutations(&headers, &mutations, false);
        assert_eq!(result.len(), 2);
        assert!(result.iter().any(|(n, v)| n == "Authorization" && v == "Bearer token"));
    }

    #[test]
    fn test_apply_mutations_remove() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("X-Debug".to_string(), "true".to_string()),
        ];
        let mutations = vec![("X-Debug".to_string(), "".to_string())];
        let result = apply_mutations(&headers, &mutations, false);
        assert_eq!(result.len(), 1);
        assert!(!result.iter().any(|(n, _)| n == "X-Debug"));
    }

    #[test]
    fn test_apply_mutations_strip_cookies() {
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Cookie".to_string(), "session=abc123".to_string()),
        ];
        let result = apply_mutations(&headers, &[], true);
        assert_eq!(result.len(), 1);
        assert!(!result.iter().any(|(n, _)| n.to_lowercase() == "cookie"));
    }
}
