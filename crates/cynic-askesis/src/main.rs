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
    /// Ingest a text file (like cortex-history.md) into the log store
    Ingest {
        /// Path to the text file to ingest
        #[arg(value_name = "INPUT_FILE")]
        input_file: PathBuf,
        /// Path to the target JSONL log file
        #[arg(short, long, value_name = "LOG_FILE")]
        logfile: PathBuf,
        /// Optional domain for the ingested entry (default: conversation)
        #[arg(short, long, default_value = "conversation")]
        domain: String,
    },
    /// Log a direct text entry to the log store
    Log {
        /// Content to log
        #[arg(short, long)]
        content: String,
        /// Path to the target JSONL log file
        #[arg(short, long, default_value = ".askesis/datasets/human-kernel.jsonl")]
        logfile: PathBuf,
        /// Domain for the entry
        #[arg(short, long)]
        domain: String,
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
        Command::Ingest {
            input_file,
            logfile,
            domain,
        } => {
            let content = std::fs::read_to_string(&input_file)
                .map_err(|e| format!("Failed to read input file {}: {e}", input_file.display()))?;

            println!("Ingesting file: {}", input_file.display());

            let mut store = JsonlLog::new(&logfile)?;

            // DE-DUPLICATION: Check if last entry is identical
            let to = chrono::Utc::now();
            let from = to - chrono::Duration::hours(1);
            if let Ok(recent) = store.range(from, to)
                && let Some(last) = recent.last()
                && last.content == content
                && last.domain.as_deref() == Some(&domain)
            {
                println!(
                    "Skipping ingestion: Content is identical to last entry (domain: {domain})."
                );
                return Ok(());
            }

            let entry = cynic_askesis::log::LogEntry::new(content).with_domain(&domain);

            store.append(entry)?;
            println!("Successfully ingested as domain: {domain}");

            Ok(())
        }
        Command::Log {
            content,
            logfile,
            domain,
        } => {
            let mut store = JsonlLog::new(&logfile)?;
            let entry = cynic_askesis::log::LogEntry::new(content).with_domain(&domain);

            store.append(entry)?;
            println!("Successfully logged to domain: {domain}");

            Ok(())
        }
    }
}
