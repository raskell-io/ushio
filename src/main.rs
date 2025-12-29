use anyhow::Result;
use clap::{Parser, Subcommand};
use std::time::Duration;

mod capture;
mod diff;
mod har;
mod output;
mod replay;

#[derive(Parser, Debug)]
#[command(name = "ushio")]
#[command(author, version)]
#[command(about = "Deterministic edge traffic replay", long_about = None)]
#[command(arg_required_else_help = true)]
#[command(after_help = "EXAMPLES:
    ushio convert session.har -o capture.json     Convert HAR to ushio format
    ushio replay capture.json -t https://staging  Replay against staging
    ushio replay capture.json -t https://prod     Replay against production
    ushio diff staging.json prod.json             Compare replay results")]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Output format
    #[arg(short, long, default_value = "pretty", global = true, value_enum)]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long, default_value = "false", global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Replay captured traffic against one or more targets
    Replay {
        /// Path to HAR file or ushio capture file
        #[arg(required = true)]
        capture: String,

        /// Target URL(s) to replay against (can specify multiple)
        #[arg(short, long, required = true)]
        target: Vec<String>,

        /// Output file for results (default: print to stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// Request timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// Mutate headers (format: "Header-Name:value" or "Header-Name:" to remove)
        #[arg(long)]
        header: Vec<String>,

        /// Strip cookies from requests
        #[arg(long, default_value = "false")]
        strip_cookies: bool,
    },

    /// Compare replay results between two targets
    Diff {
        /// First replay result file
        #[arg(required = true)]
        left: String,

        /// Second replay result file
        #[arg(required = true)]
        right: String,

        /// Only show differences
        #[arg(long, default_value = "false")]
        only_diff: bool,
    },

    /// Convert HAR file to ushio capture format
    Convert {
        /// Input HAR file
        #[arg(required = true)]
        input: String,

        /// Output file (default: stdout)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
enum OutputFormat {
    Pretty,
    Json,
    Compact,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(if args.verbose {
                    tracing::Level::DEBUG.into()
                } else {
                    tracing::Level::INFO.into()
                }),
        )
        .init();

    match args.command {
        Command::Replay {
            capture,
            target,
            output,
            timeout,
            header,
            strip_cookies,
        } => {
            // Load capture (try as ushio format first, then HAR)
            let requests = load_capture_or_har(&capture)?;

            // Parse header mutations
            let header_mutations: Vec<(String, String)> = header
                .iter()
                .filter_map(|h| {
                    let parts: Vec<&str> = h.splitn(2, ':').collect();
                    if parts.len() == 2 {
                        Some((parts[0].to_string(), parts[1].to_string()))
                    } else if parts.len() == 1 && h.ends_with(':') {
                        Some((parts[0].to_string(), String::new()))
                    } else {
                        eprintln!("Warning: Invalid header format '{}', expected 'Name:value'", h);
                        None
                    }
                })
                .collect();

            let config = replay::ReplayConfig {
                timeout: Duration::from_secs(timeout),
                concurrency: 1,
                header_mutations,
                strip_cookies,
            };

            // Replay against each target
            for t in &target {
                let session = replay::replay(&requests, t, config.clone()).await?;

                // Output results
                match args.format {
                    OutputFormat::Pretty => {
                        output::print_replay_pretty(&session);
                    }
                    OutputFormat::Json => {
                        println!("{}", output::print_replay_json(&session));
                    }
                    OutputFormat::Compact => {
                        println!("{}", output::print_replay_compact(&session));
                    }
                }

                // Save to file if requested
                if let Some(ref path) = output {
                    let output_path = if target.len() > 1 {
                        // Add target suffix for multiple targets
                        let suffix = t.replace("://", "_").replace(['/', ':'], "_");
                        format!("{}_{}", path.trim_end_matches(".json"), suffix)
                    } else {
                        path.clone()
                    };
                    replay::save_session(&session, &output_path)?;
                    eprintln!("Saved results to {}", output_path);
                }
            }
        }

        Command::Diff {
            left,
            right,
            only_diff,
        } => {
            // Load sessions
            let left_session = replay::load_session(&left)?;
            let right_session = replay::load_session(&right)?;

            // Compute diff
            let summary = diff::diff_sessions(&left_session, &right_session);

            // Output
            match args.format {
                OutputFormat::Pretty => {
                    output::print_diff_pretty(&summary, only_diff);
                }
                OutputFormat::Json => {
                    println!("{}", output::print_diff_json(&summary));
                }
                OutputFormat::Compact => {
                    println!("{}", output::print_diff_compact(&summary));
                }
            }

            // Exit with code 1 if there are differences
            if summary.different > 0 {
                std::process::exit(1);
            }
        }

        Command::Convert { input, output } => {
            // Read HAR file
            let content = std::fs::read_to_string(&input)
                .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", input, e))?;

            // Parse HAR
            let har_data = har::parse_har(&content)
                .map_err(|e| anyhow::anyhow!("Failed to parse HAR: {}", e))?;

            // Convert to capture format
            let requests = har::har_to_capture(har_data);
            let capture_data = capture::Capture::new(requests).with_source(input.clone());

            // Output
            let json = serde_json::to_string_pretty(&capture_data)?;
            match output {
                Some(path) => {
                    std::fs::write(&path, &json)?;
                    eprintln!("Converted {} requests to {}", capture_data.requests.len(), path);
                }
                None => {
                    println!("{}", json);
                }
            }
        }
    }

    Ok(())
}

/// Load requests from either ushio capture format or HAR
fn load_capture_or_har(path: &str) -> Result<Vec<capture::CapturedRequest>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", path, e))?;

    // Try as ushio capture first
    if let Ok(cap) = serde_json::from_str::<capture::Capture>(&content) {
        return Ok(cap.requests);
    }

    // Try as HAR
    if let Ok(har_data) = har::parse_har(&content) {
        return Ok(har::har_to_capture(har_data));
    }

    Err(anyhow::anyhow!(
        "Failed to parse {} as either ushio capture or HAR format",
        path
    ))
}
