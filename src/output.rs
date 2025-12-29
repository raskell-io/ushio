//! Output formatting for replay and diff results
//!
//! Supports pretty (colored terminal), JSON, and compact formats.

use colored::Colorize;

use crate::diff::{DiffSummary, HeaderDiffType, RequestDiff};
use crate::replay::ReplaySession;

/// Print replay session in pretty format
pub fn print_replay_pretty(session: &ReplaySession) {
    println!();
    println!(
        "{} {}",
        "ushio".bold().cyan(),
        "traffic replay".dimmed()
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    // Summary
    println!("  {} {}", "Target:".bold(), session.target);
    println!(
        "  {} {}",
        "Time:".bold(),
        session.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
    );
    println!();

    // Stats
    println!("  {} {}", "Requests:".bold(), session.total_requests);
    println!(
        "  {} {}",
        "Successful:".bold(),
        session.successful.to_string().green()
    );
    if session.failed > 0 {
        println!(
            "  {} {}",
            "Failed:".bold(),
            session.failed.to_string().red()
        );
    }
    if session.status_mismatches > 0 {
        println!(
            "  {} {}",
            "Mismatches:".bold(),
            session.status_mismatches.to_string().yellow()
        );
    }
    println!();

    // Show mismatches and errors
    let issues: Vec<_> = session
        .results
        .iter()
        .filter(|r| !r.status_match || r.error.is_some())
        .collect();

    if !issues.is_empty() {
        println!("  {}", "Issues".bold().underline());
        println!();

        for result in issues {
            let status_str = if result.error.is_some() {
                "ERR".red().to_string()
            } else if result.status >= 400 {
                result.status.to_string().red().to_string()
            } else {
                result.status.to_string().yellow().to_string()
            };

            println!(
                "    {} {} {}",
                format!("#{}", result.request_index).dimmed(),
                result.method.bold(),
                truncate_url(&result.url, 40)
            );

            if let Some(ref error) = result.error {
                println!("      {} {}", "Error:".red(), error);
            } else {
                let expected = result
                    .expected_status
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "?".to_string());
                println!(
                    "      Expected: {}, Got: {}",
                    expected.green(),
                    status_str
                );
            }
            println!();
        }
    }

    println!("{}", "─".repeat(60).dimmed());
}

/// Print diff summary in pretty format
pub fn print_diff_pretty(summary: &DiffSummary, only_diff: bool) {
    println!();
    println!(
        "{} {}",
        "ushio".bold().cyan(),
        "diff results".dimmed()
    );
    println!("{}", "─".repeat(60).dimmed());
    println!();

    // Targets
    println!("  {} {}", "Left:".bold(), summary.left_target);
    println!("  {} {}", "Right:".bold(), summary.right_target);
    println!();

    // Stats
    println!("  {} {}", "Total:".bold(), summary.total_requests);
    println!(
        "  {} {}",
        "Identical:".bold(),
        summary.identical.to_string().green()
    );
    if summary.different > 0 {
        println!(
            "  {} {}",
            "Different:".bold(),
            summary.different.to_string().yellow()
        );
    }
    if summary.waf_diffs > 0 {
        println!(
            "  {} {}",
            "WAF diffs:".bold(),
            summary.waf_diffs.to_string().red()
        );
    }
    println!();

    // Show differences
    if !summary.diffs.is_empty() {
        println!("  {}", "Differences".bold().underline());
        println!();

        for diff in &summary.diffs {
            print_request_diff(diff);
        }
    } else if !only_diff {
        println!("  {} No differences found", "✓".green());
        println!();
    }

    println!("{}", "─".repeat(60).dimmed());
}

/// Print a single request diff
fn print_request_diff(diff: &RequestDiff) {
    println!(
        "    {} {} {}",
        format!("#{}", diff.request_index).dimmed(),
        diff.method.bold(),
        truncate_url(&diff.url, 40)
    );

    // Status diff
    if let Some(ref status) = diff.status_diff {
        let left_str = format_status(status.left);
        let right_str = format_status(status.right);
        println!("      {} {} → {}", "Status:".dimmed(), left_str, right_str);
    }

    // WAF diff
    if let Some(ref waf) = diff.waf_diff {
        let left_str = if waf.left_blocked {
            "blocked".red().to_string()
        } else {
            "allowed".green().to_string()
        };
        let right_str = if waf.right_blocked {
            "blocked".red().to_string()
        } else {
            "allowed".green().to_string()
        };
        println!("      {} {} → {}", "WAF:".dimmed(), left_str, right_str);

        if let Some(ref reason) = waf.left_reason {
            println!("        {} {}", "Left:".dimmed(), reason);
        }
        if let Some(ref reason) = waf.right_reason {
            println!("        {} {}", "Right:".dimmed(), reason);
        }
    }

    // Header diffs
    for header in &diff.header_diffs {
        let change = match header.diff_type {
            HeaderDiffType::Added => "+".green().to_string(),
            HeaderDiffType::Removed => "-".red().to_string(),
            HeaderDiffType::Changed => "~".yellow().to_string(),
        };

        let left = header.left.as_deref().unwrap_or("-");
        let right = header.right.as_deref().unwrap_or("-");

        println!(
            "      {} {} {} → {}",
            change,
            header.name.dimmed(),
            truncate(left, 20),
            truncate(right, 20)
        );
    }

    println!();
}

/// Format status code with color
fn format_status(status: u16) -> String {
    if status == 0 {
        "N/A".dimmed().to_string()
    } else if status >= 500 {
        status.to_string().red().to_string()
    } else if status >= 400 {
        status.to_string().yellow().to_string()
    } else if status >= 300 {
        status.to_string().cyan().to_string()
    } else {
        status.to_string().green().to_string()
    }
}

/// Print replay session as JSON
pub fn print_replay_json(session: &ReplaySession) -> String {
    serde_json::to_string_pretty(session).unwrap_or_else(|_| "{}".to_string())
}

/// Print diff summary as JSON
pub fn print_diff_json(summary: &DiffSummary) -> String {
    serde_json::to_string_pretty(summary).unwrap_or_else(|_| "{}".to_string())
}

/// Print replay session in compact format
pub fn print_replay_compact(session: &ReplaySession) -> String {
    let mut parts = vec![format!("{}: {}/{}", session.target, session.successful, session.total_requests)];

    if session.failed > 0 {
        parts.push(format!("failed={}", session.failed));
    }
    if session.status_mismatches > 0 {
        parts.push(format!("mismatches={}", session.status_mismatches));
    }

    parts.join(" ")
}

/// Print diff summary in compact format
pub fn print_diff_compact(summary: &DiffSummary) -> String {
    let status = if summary.different == 0 {
        "SAME"
    } else if summary.waf_diffs > 0 {
        "WAF_DIFF"
    } else {
        "DIFF"
    };

    format!(
        "{} vs {}: {} identical={} different={} waf={}",
        summary.left_target,
        summary.right_target,
        status,
        summary.identical,
        summary.different,
        summary.waf_diffs
    )
}

/// Truncate a string
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len - 3])
    } else {
        s.to_string()
    }
}

/// Truncate URL, keeping the path visible
fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        return url.to_string();
    }

    // Try to extract just the path
    if let Ok(parsed) = url::Url::parse(url) {
        let path = parsed.path();
        if path.len() <= max_len {
            return format!("...{}", path);
        }
        return format!("...{}", &path[path.len() - max_len + 3..]);
    }

    truncate(url, max_len)
}
