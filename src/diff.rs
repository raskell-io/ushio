//! Behavioral diff analysis
//!
//! Compares replay results between two targets to identify differences
//! in status codes, headers, and WAF decisions.

use serde::{Deserialize, Serialize};

use crate::replay::{ReplayResult, ReplaySession};

/// Difference between two replay results
#[derive(Debug, Serialize, Deserialize)]
pub struct RequestDiff {
    pub request_index: usize,
    pub method: String,
    pub url: String,
    pub status_diff: Option<StatusDiff>,
    pub header_diffs: Vec<HeaderDiff>,
    pub waf_diff: Option<WafDiff>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusDiff {
    pub left: u16,
    pub right: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeaderDiff {
    pub name: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub diff_type: HeaderDiffType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HeaderDiffType {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WafDiff {
    pub left_blocked: bool,
    pub right_blocked: bool,
    pub left_reason: Option<String>,
    pub right_reason: Option<String>,
}

/// Summary of differences between two replay sessions
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffSummary {
    pub left_target: String,
    pub right_target: String,
    pub total_requests: usize,
    pub identical: usize,
    pub different: usize,
    pub status_diffs: usize,
    pub header_diffs: usize,
    pub waf_diffs: usize,
    pub diffs: Vec<RequestDiff>,
}

/// Headers to compare for differences (WAF-related and security headers)
const COMPARE_HEADERS: &[&str] = &[
    "x-waf-action",
    "x-waf-rule",
    "x-waf-score",
    "cf-ray",
    "cf-cache-status",
    "x-cache",
    "x-cache-status",
    "x-blocked",
    "x-blocked-by",
    "server",
    "x-frame-options",
    "content-security-policy",
    "strict-transport-security",
    "x-content-type-options",
];

/// Compare two replay sessions and produce a diff summary
pub fn diff_sessions(left: &ReplaySession, right: &ReplaySession) -> DiffSummary {
    let mut diffs = Vec::new();
    let mut identical = 0;
    let mut different = 0;
    let mut status_diffs_count = 0;
    let mut header_diffs_count = 0;
    let mut waf_diffs_count = 0;

    // Match requests by index
    let max_len = left.results.len().max(right.results.len());

    for i in 0..max_len {
        let left_result = left.results.get(i);
        let right_result = right.results.get(i);

        match (left_result, right_result) {
            (Some(l), Some(r)) => {
                if let Some(diff) = diff_results(l, r) {
                    if diff.status_diff.is_some() {
                        status_diffs_count += 1;
                    }
                    if !diff.header_diffs.is_empty() {
                        header_diffs_count += 1;
                    }
                    if diff.waf_diff.is_some() {
                        waf_diffs_count += 1;
                    }
                    different += 1;
                    diffs.push(diff);
                } else {
                    identical += 1;
                }
            }
            (Some(l), None) => {
                // Right side missing
                different += 1;
                diffs.push(RequestDiff {
                    request_index: i,
                    method: l.method.clone(),
                    url: l.url.clone(),
                    status_diff: Some(StatusDiff {
                        left: l.status,
                        right: 0,
                    }),
                    header_diffs: vec![],
                    waf_diff: None,
                });
            }
            (None, Some(r)) => {
                // Left side missing
                different += 1;
                diffs.push(RequestDiff {
                    request_index: i,
                    method: r.method.clone(),
                    url: r.url.clone(),
                    status_diff: Some(StatusDiff {
                        left: 0,
                        right: r.status,
                    }),
                    header_diffs: vec![],
                    waf_diff: None,
                });
            }
            (None, None) => {
                // Should not happen
            }
        }
    }

    DiffSummary {
        left_target: left.target.clone(),
        right_target: right.target.clone(),
        total_requests: max_len,
        identical,
        different,
        status_diffs: status_diffs_count,
        header_diffs: header_diffs_count,
        waf_diffs: waf_diffs_count,
        diffs,
    }
}

/// Compare two individual replay results
pub fn diff_results(left: &ReplayResult, right: &ReplayResult) -> Option<RequestDiff> {
    let status_diff = if left.status != right.status {
        Some(StatusDiff {
            left: left.status,
            right: right.status,
        })
    } else {
        None
    };

    let header_diffs = diff_headers(&left.headers, &right.headers);
    let waf_diff = detect_waf_diff(left, right);

    // Only return a diff if there are actual differences
    if status_diff.is_none() && header_diffs.is_empty() && waf_diff.is_none() {
        return None;
    }

    Some(RequestDiff {
        request_index: left.request_index,
        method: left.method.clone(),
        url: left.url.clone(),
        status_diff,
        header_diffs,
        waf_diff,
    })
}

/// Compare headers between two responses
fn diff_headers(left: &[(String, String)], right: &[(String, String)]) -> Vec<HeaderDiff> {
    let mut diffs = Vec::new();

    for header_name in COMPARE_HEADERS {
        let left_value = find_header(left, header_name);
        let right_value = find_header(right, header_name);

        match (&left_value, &right_value) {
            (Some(l), Some(r)) if l != r => {
                diffs.push(HeaderDiff {
                    name: header_name.to_string(),
                    left: Some(l.clone()),
                    right: Some(r.clone()),
                    diff_type: HeaderDiffType::Changed,
                });
            }
            (Some(l), None) => {
                diffs.push(HeaderDiff {
                    name: header_name.to_string(),
                    left: Some(l.clone()),
                    right: None,
                    diff_type: HeaderDiffType::Removed,
                });
            }
            (None, Some(r)) => {
                diffs.push(HeaderDiff {
                    name: header_name.to_string(),
                    left: None,
                    right: Some(r.clone()),
                    diff_type: HeaderDiffType::Added,
                });
            }
            _ => {}
        }
    }

    diffs
}

/// Find a header value by name (case-insensitive)
fn find_header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.clone())
}

/// Detect WAF-related differences based on status codes and headers
fn detect_waf_diff(left: &ReplayResult, right: &ReplayResult) -> Option<WafDiff> {
    let left_blocked = is_waf_block(left);
    let right_blocked = is_waf_block(right);

    // Only report if blocking status differs
    if left_blocked == right_blocked {
        return None;
    }

    Some(WafDiff {
        left_blocked,
        right_blocked,
        left_reason: get_waf_reason(left),
        right_reason: get_waf_reason(right),
    })
}

/// Check if a response indicates a WAF block
fn is_waf_block(result: &ReplayResult) -> bool {
    // Status codes that typically indicate blocking
    if matches!(result.status, 403 | 429 | 503) {
        return true;
    }

    // Check for WAF-specific headers
    let waf_header_prefixes = ["x-waf-", "x-blocked"];
    for (name, _) in &result.headers {
        let name_lower = name.to_lowercase();
        if waf_header_prefixes
            .iter()
            .any(|prefix| name_lower.starts_with(prefix))
        {
            return true;
        }
    }

    false
}

/// Extract WAF reason from headers
fn get_waf_reason(result: &ReplayResult) -> Option<String> {
    // Try common WAF reason headers
    let reason_headers = ["x-waf-rule", "x-waf-action", "x-blocked-by", "x-blocked"];

    for header in reason_headers {
        if let Some(value) = find_header(&result.headers, header) {
            return Some(format!("{}: {}", header, value));
        }
    }

    // Fall back to status code
    if matches!(result.status, 403 | 429 | 503) {
        return Some(format!("HTTP {}", result.status));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(index: usize, status: u16, headers: Vec<(&str, &str)>) -> ReplayResult {
        ReplayResult {
            request_index: index,
            method: "GET".to_string(),
            url: "https://example.com/test".to_string(),
            status,
            headers: headers
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            body_size: 0,
            duration_ms: 100,
            expected_status: Some(200),
            status_match: status == 200,
            error: None,
        }
    }

    #[test]
    fn test_diff_identical() {
        let left = make_result(0, 200, vec![("content-type", "application/json")]);
        let right = make_result(0, 200, vec![("content-type", "application/json")]);
        assert!(diff_results(&left, &right).is_none());
    }

    #[test]
    fn test_diff_status() {
        let left = make_result(0, 200, vec![]);
        let right = make_result(0, 403, vec![]);
        let diff = diff_results(&left, &right).unwrap();
        assert!(diff.status_diff.is_some());
        assert_eq!(diff.status_diff.as_ref().unwrap().left, 200);
        assert_eq!(diff.status_diff.as_ref().unwrap().right, 403);
    }

    #[test]
    fn test_waf_block_detection() {
        let blocked = make_result(0, 403, vec![("x-waf-rule", "942100")]);
        let allowed = make_result(0, 200, vec![]);
        assert!(is_waf_block(&blocked));
        assert!(!is_waf_block(&allowed));
    }

    #[test]
    fn test_waf_diff() {
        let left = make_result(0, 200, vec![]);
        let right = make_result(0, 403, vec![("x-waf-rule", "942100")]);
        let diff = diff_results(&left, &right).unwrap();
        assert!(diff.waf_diff.is_some());
        let waf = diff.waf_diff.unwrap();
        assert!(!waf.left_blocked);
        assert!(waf.right_blocked);
    }
}
