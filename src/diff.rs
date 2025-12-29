//! Behavioral diff analysis
//!
//! Compares replay results between two targets to identify differences
//! in status codes, headers, and WAF decisions.

use serde::Serialize;

use crate::replay::{ReplayResult, ReplaySession};

/// Difference between two replay results
#[derive(Debug, Serialize)]
pub struct RequestDiff {
    pub request_index: usize,
    pub url: String,
    pub status_diff: Option<StatusDiff>,
    pub header_diffs: Vec<HeaderDiff>,
    pub waf_diff: Option<WafDiff>,
}

#[derive(Debug, Serialize)]
pub struct StatusDiff {
    pub left: u16,
    pub right: u16,
}

#[derive(Debug, Serialize)]
pub struct HeaderDiff {
    pub name: String,
    pub left: Option<String>,
    pub right: Option<String>,
    pub diff_type: HeaderDiffType,
}

#[derive(Debug, Serialize)]
pub enum HeaderDiffType {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Serialize)]
pub struct WafDiff {
    pub left_blocked: bool,
    pub right_blocked: bool,
    pub left_reason: Option<String>,
    pub right_reason: Option<String>,
}

/// Summary of differences between two replay sessions
#[derive(Debug, Serialize)]
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

/// Compare two replay sessions and produce a diff summary
pub fn diff_sessions(_left: &ReplaySession, _right: &ReplaySession) -> DiffSummary {
    // TODO: Implement diff logic
    todo!("Diff logic not yet implemented")
}

/// Compare two individual replay results
pub fn diff_results(_left: &ReplayResult, _right: &ReplayResult) -> Option<RequestDiff> {
    // TODO: Implement result comparison
    todo!("Result comparison not yet implemented")
}

/// Detect WAF-related differences based on status codes and headers
fn _detect_waf_diff(_left: &ReplayResult, _right: &ReplayResult) -> Option<WafDiff> {
    // TODO: Implement WAF diff detection
    // Look for patterns like:
    // - 403 vs 200 (blocked vs allowed)
    // - WAF-specific headers (X-WAF-*, etc.)
    todo!("WAF diff detection not yet implemented")
}
