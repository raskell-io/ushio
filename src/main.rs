use anyhow::Result;
use clap::{Parser, Subcommand};

mod capture;
mod diff;
mod har;
mod replay;

#[derive(Parser, Debug)]
#[command(name = "ushio")]
#[command(author, version, about = "Deterministic edge traffic replay", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Command,

    /// Output format
    #[arg(short, long, default_value = "pretty", global = true)]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long, default_value = "false", global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Replay captured traffic against one or more targets
    Replay {
        /// Path to HAR file or capture directory
        #[arg(required = true)]
        capture: String,

        /// Target URL(s) to replay against (can specify multiple)
        #[arg(short, long, required = true)]
        target: Vec<String>,

        /// Request timeout in seconds
        #[arg(long, default_value = "30")]
        timeout: u64,

        /// Number of concurrent requests
        #[arg(long, default_value = "1")]
        concurrency: usize,

        /// Mutate headers (format: "Header-Name:value")
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
        Command::Replay { capture, target, .. } => {
            println!("ushio - traffic replay");
            println!("Capture: {}", capture);
            println!("Targets: {:?}", target);
            println!();
            println!("Coming soon: deterministic replay with diff support");
        }
        Command::Diff { left, right, .. } => {
            println!("ushio - diff results");
            println!("Left: {}", left);
            println!("Right: {}", right);
            println!();
            println!("Coming soon: behavioral diff analysis");
        }
        Command::Convert { input, output } => {
            println!("ushio - convert HAR");
            println!("Input: {}", input);
            println!("Output: {:?}", output.unwrap_or_else(|| "stdout".to_string()));
            println!();
            println!("Coming soon: HAR to ushio format conversion");
        }
    }

    Ok(())
}
