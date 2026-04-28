// WHY: CLI binary must output to stdout via println!. Not debug code, production output.
#![allow(clippy::print_stdout)]

use clap::Parser;
use cynic_askesis::audit::gemini_wisdom::GeminiWisdomAudit;
use cynic_askesis::audit::{AuditEngine, default_phase2_directives};
use cynic_askesis::log::LogStore;
use cynic_askesis::log::jsonl::JsonlLog;
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
            let log_path = logfile
                .canonicalize()
                .map_err(|e| format!("Failed to resolve log path {}: {e}", logfile.display()))?;

            println!("Auditing log: {}", log_path.display());

            // Initialize log store and load entries for the last 7 days
            let store = JsonlLog::new(log_path)?;
            let to = chrono::Utc::now();
            let from = to - chrono::Duration::days(7);
            let logs = store.range(from, to)?;

            if logs.is_empty() {
                println!("No log entries found in the last 7 days.");
                return Ok(());
            }

            println!("Found {} entries. Running Gemini audit...", logs.len());

            // Run audit using Gemini Wisdom
            let engine = GeminiWisdomAudit::default();
            let directives = default_phase2_directives();
            let reflection = engine.audit(&logs, &directives).await?;

            println!("\n=== REFLECTION ===");
            println!("{}", serde_json::to_string_pretty(&reflection)?);
            println!("\n=== MARKDOWN ===");
            println!("{}", reflection.to_markdown());

            Ok(())
        }
    }
}
