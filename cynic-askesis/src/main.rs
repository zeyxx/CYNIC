// WHY: CLI binary must output to stdout via println!. Not debug code, production output.
#![allow(clippy::print_stdout)]

use clap::Parser;
use cynic_askesis::{Reflection, Verdict};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "cynic-askesis")]
#[command(about = "Askesis: discipline through exercise")]
#[command(version)]
enum Command {
    /// Audit a JSONL log file and produce a verdict
    Audit {
        /// Path to JSONL log file
        #[arg(value_name = "FILE")]
        logfile: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cmd = Command::parse();

    match cmd {
        Command::Audit { logfile } => {
            // Phase 1: Load log, audit, output verdict.
            // Future: integrate with real LogStore backends and DomainTracker.

            let log_path = logfile
                .canonicalize()
                .map_err(|e| format!("Failed to resolve log path: {e}"))?;

            println!("Auditing log: {}", log_path.display());

            // Phase 1: Return placeholder reflection
            let reflection = Reflection {
                verdict: Verdict::Wag,
                prose: "CLI skeleton operational — Phase 1 ready for integration".to_string(),
                patterns_detected: vec!["cli-ready".to_string()],
                kenosis_candidate: None,
                confidence: 0.618,
            };

            println!("\n=== REFLECTION ===");
            println!("{}", serde_json::to_string_pretty(&reflection)?);
            println!("\n=== MARKDOWN ===");
            println!("{}", reflection.to_markdown());

            Ok(())
        }
    }
}
