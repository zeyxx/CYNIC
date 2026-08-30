//! Memory surface — operator CLI commands for Askēsis observation log auditing,
//! Proof-of-History chain verification, compaction, and vector indexing.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_askesis::log::jsonl::{JsonlLog, LogRetentionPolicy};
use cynic_askesis::memory::resolve_human_log_path;
use cynic_inference::domain::embedding::Embedding;
use cynic_inference::vector_memory::VectorMemoryStore;

#[derive(Subcommand)]
pub(crate) enum MemoryCommand {
    /// Verify the Proof-of-History cryptographic hash chain integrity of the Askēsis log.
    Verify {
        /// Optional path to the JSONL log file (defaults to ~/.cynic/memory/logs/human-kernel.jsonl).
        #[arg(long)]
        path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Compact the Askēsis log according to retention policy, archiving older entries.
    Compact {
        /// Optional path to the JSONL log file (defaults to ~/.cynic/memory/logs/human-kernel.jsonl).
        #[arg(long)]
        path: Option<String>,
        /// Maximum active entries to keep in the hot log (default: 500).
        #[arg(long, default_value = "500")]
        max_entries: usize,
        /// Directory where archived segments are written.
        #[arg(long)]
        archive_dir: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Index an archive JSONL file into the semantic vector store.
    Index {
        /// Path to the archive JSONL file to index.
        #[arg(long)]
        archive_path: String,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_memory(command: MemoryCommand) -> ExitCode {
    match command {
        MemoryCommand::Verify { path, json } => {
            let log_path = path
                .map(PathBuf::from)
                .unwrap_or_else(resolve_human_log_path);
            let log = match JsonlLog::new(&log_path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to open log at {}: {e}", log_path.display());
                    return ExitCode::FAILURE;
                }
            };
            match log.verify_chain() {
                Ok(result) => {
                    if json {
                        crate::cli_json::emit_pretty_json(&result)
                    } else if result.is_valid {
                        println!(
                            "PoH Chain Valid: {} entries verified. Head hash: {}",
                            result.valid_entries,
                            result.head_hash.as_deref().unwrap_or("none")
                        );
                        ExitCode::SUCCESS
                    } else {
                        eprintln!(
                            "PoH Chain Broken! {} valid entries, violations: {:?}",
                            result.valid_entries, result.violations
                        );
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("Verification failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        MemoryCommand::Compact {
            path,
            max_entries,
            archive_dir,
            json,
        } => {
            let log_path = path
                .map(PathBuf::from)
                .unwrap_or_else(resolve_human_log_path);
            let mut log = match JsonlLog::new(&log_path) {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Failed to open log at {}: {e}", log_path.display());
                    return ExitCode::FAILURE;
                }
            };
            let policy = LogRetentionPolicy {
                max_active_entries: max_entries,
                max_age_ms: None,
                archive_dir: archive_dir.map(PathBuf::from),
            };
            match log.compact(&policy) {
                Ok(report) => {
                    if json {
                        crate::cli_json::emit_pretty_json(&report)
                    } else {
                        println!(
                            "Compaction complete: {} active retained, {} archived.",
                            report.active_count, report.archived_count
                        );
                        if let Some(archive) = report.archive_path {
                            println!("Archive written to: {}", archive.display());
                        }
                        if let Some(anchor) = report.anchor_hash {
                            println!("Anchor hash: {}", anchor);
                        }
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Compaction failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        MemoryCommand::Index { archive_path, json } => {
            let path = PathBuf::from(archive_path);
            let mut store = VectorMemoryStore::new();
            // Deterministic character-frequency hash embedding for local CLI indexing demonstration
            let fake_embedder = |text: &str| {
                let mut v = vec![0.0f32; 16];
                for (i, b) in text.bytes().enumerate() {
                    v[i % 16] += b as f32 / 255.0;
                }
                Embedding {
                    vector: v,
                    dimensions: 16,
                    prompt_tokens: text.split_whitespace().count() as u32,
                }
            };
            match store.index_jsonl_archive(&path, fake_embedder) {
                Ok(report) => {
                    if json {
                        crate::cli_json::emit_pretty_json(&report)
                    } else {
                        println!(
                            "Indexed archive {}: {} events indexed, {} skipped (total store: {})",
                            report.archive_path,
                            report.indexed_events,
                            report.skipped_events,
                            report.total_store_size
                        );
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Indexing failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
