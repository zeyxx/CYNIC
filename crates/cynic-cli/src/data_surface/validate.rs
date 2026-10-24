//! Tabular schema validation and schema template emission.

use std::path::Path;
use std::process::ExitCode;

use cynic_protocol::DataReport;
use cynic_protocol::tabular::{
    DriftSeverity, SchemaDriftKind, TabularSchema,
};
use super::history::append_history_report;

pub fn execute_validate(schema_path: &Path, file_path: &Path, json: bool, record: bool) -> ExitCode {
    let schema: TabularSchema = if schema_path.exists() && schema_path.is_file() {
        let schema_str = match std::fs::read_to_string(schema_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Cannot read schema file '{}': {}", schema_path.display(), e);
                return ExitCode::FAILURE;
            }
        };
        match serde_json::from_str(&schema_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: Invalid schema JSON in '{}': {}", schema_path.display(), e);
                return ExitCode::FAILURE;
            }
        }
    } else {
        match super::query::resolve_schema(Some(&schema_path.to_string_lossy())) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: Cannot resolve schema from '{}'. Expected valid JSON schema file or canonical name ('session', 'turn', 'asset', 'natcat', 'chunk', 'claim', 'history', 'journal', 'telemetry').", schema_path.display());
                return ExitCode::FAILURE;
            }
        }
    };

    let content = match std::fs::read_to_string(file_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Cannot read data file '{}': {}", file_path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let report = match ext.as_str() {
        "csv" => cynic_traces::tabular::validate_csv_content(&schema, &content),
        "jsonl" | "ndjson" => cynic_traces::tabular::validate_jsonl_content(&schema, &content),
        "json" => cynic_traces::tabular::validate_json_content(&schema, &content),
        _ => {
            let trimmed = content.trim_start();
            if trimmed.starts_with('{') || trimmed.starts_with('[') {
                cynic_traces::tabular::validate_json_content(&schema, &content)
            } else {
                cynic_traces::tabular::validate_csv_content(&schema, &content)
            }
        }
    };

    if record {
        let history_path = Path::new("evidence/data_validation_history.jsonl");
        if let Err(e) = append_history_report(&report, history_path) {
            eprintln!("Warning: Failed to record validation run to evidence history: {}", e);
        } else if !json {
            println!("Recorded to   : {}", history_path.display());
        }

        // Cryptographic sealing in EvidenceJournalStore
        let evidence_root = Path::new("evidence");
        match cynic_storage::evidence_journal_file::EvidenceJournalStore::open(evidence_root, "local-node") {
            Ok(store) => {
                if let Ok(report_bytes) = serde_json::to_vec(&report) {
                    let digest = cynic_protocol::digest::digest_bytes(&report_bytes);
                    let now = chrono::Utc::now().timestamp();
                    match store.append("tabular.validation", &digest, now) {
                        Ok(outcome) => {
                            if !json {
                                println!(
                                    "Sealed Proof  : {} (seq #{} | hash: {})",
                                    outcome.segment_path.display(),
                                    outcome.sequence,
                                    outcome.entry_hash
                                );
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to seal evidence journal entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Cannot open evidence journal store: {}", e);
            }
        }
    }

    if json {
        match report.to_export_json() {
            Ok(json_output) => println!("{}", json_output),
            Err(e) => {
                eprintln!("Error serializing report to JSON: {}", e);
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("=== CYNIC Tabular Data Validation ===");
        println!("Target Schema : {} (v{})", report.target_schema_id, report.target_schema_version);
        println!("Source File   : {}", file_path.display());
        println!("Total Rows    : {}", report.total_rows);
        println!("Valid Rows    : {}", report.valid_rows);
        println!("Invalid Rows  : {}", report.invalid_rows);
        println!("Status        : {}", if report.passed { "PASSED" } else { "FAILED" });

        if !report.drifts.is_empty() {
            println!();
            println!("Detected Drifts / Violations ({} total):", report.drifts.len());
            for (idx, drift) in report.drifts.iter().enumerate().take(20) {
                let row_str = match drift.row_index {
                    Some(r) => format!("row {}", r),
                    None => "dataset".to_string(),
                };
                let sev = match drift.severity {
                    DriftSeverity::Critical => "CRITICAL",
                    DriftSeverity::Warning => "WARN",
                };
                match &drift.drift {
                    SchemaDriftKind::ColumnMissing { column } => {
                        println!("  - [{}] [{}] Missing required column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::UnexpectedColumn { column } => {
                        println!("  - [{}] [{}] Unexpected column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::TypeMismatch { column, expected, found_sample } => {
                        println!(
                            "  - [{}] [{}] Type mismatch for '{}': expected {}, observed '{}'",
                            sev, row_str, column, expected, found_sample
                        );
                    }
                    SchemaDriftKind::NullViolation { column } => {
                        println!("  - [{}] [{}] Column '{}' is null or empty but marked required", sev, row_str, column);
                    }
                    SchemaDriftKind::MalformedRecord { message } => {
                        println!("  - [{}] [{}] Malformed record: {}", sev, row_str, message);
                    }
                    SchemaDriftKind::ValueOutOfRange { column, value, min, max } => {
                        println!(
                            "  - [{}] [{}] Range violation for '{}': value {} outside [{:?}, {:?}]",
                            sev, row_str, column, value, min, max
                        );
                    }
                    SchemaDriftKind::ValueNotAllowed { column, value, allowed } => {
                        println!(
                            "  - [{}] [{}] Value '{}' for '{}' not in allowed values {:?}",
                            sev, row_str, value, column, allowed
                        );
                    }
                    SchemaDriftKind::PatternMismatch { column, value, pattern } => {
                        println!(
                            "  - [{}] [{}] Value '{}' for '{}' does not match regex '{}'",
                            sev, row_str, value, column, pattern
                        );
                    }
                }
                if idx == 19 && report.drifts.len() > 20 {
                    println!("  ... and {} more drifts omitted.", report.drifts.len() - 20);
                }
            }
        }
    }

    if report.passed {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

pub fn execute_schema(kind: &str) -> ExitCode {
    let schema = match kind.to_lowercase().as_str() {
        "asset" | "assets" | "node_inventory" | "inventory" => cynic_traces::tabular::node_inventory_schema(),
        "claim" | "claims" | "epistemic_claim" | "fact" => cynic_traces::tabular::epistemic_claim_schema(),
        "chunk" | "chunks" | "document_chunk" => cynic_traces::tabular::document_chunk_schema(),
        "natcat" | "insurance" => cynic_traces::tabular::natcat_claims_schema(),
        "session" | "sessions" | "session_summary" => cynic_traces::tabular::session_summary_schema(),
        "turn" | "turns" | "session_turn" => cynic_traces::tabular::session_turn_schema(),
        "history" | "reports" | "report" | "validation" => cynic_traces::tabular::tabular_validation_report_schema(),
        "journal" | "proofs" | "proof" | "evidence" => cynic_traces::tabular::evidence_journal_entry_schema(),
        "telemetry" | "node_telemetry" | "heartbeat" => cynic_traces::tabular::node_telemetry_schema(),
        _ => cynic_traces::tabular::session_summary_schema(),
    };

    match serde_json::to_string_pretty(&schema) {
        Ok(json) => {
            println!("{}", json);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("Error serializing sample schema: {}", e);
            ExitCode::FAILURE
        }
    }
}
