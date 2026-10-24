//! Tabular schema validation and schema template emission.

use std::path::Path;
use std::process::ExitCode;

use cynic_protocol::tabular::{
    DriftSeverity, SchemaDriftKind, TabularSchema, TabularValidationReport,
};
use super::history::append_history_report;

pub fn execute_validate(schema_path: &Path, file_path: &Path, json: bool, record: bool) -> ExitCode {
    let schema_str = match std::fs::read_to_string(schema_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Cannot read schema file '{}': {}", schema_path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    let schema: TabularSchema = match serde_json::from_str(&schema_str) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: Invalid schema JSON in '{}': {}", schema_path.display(), e);
            return ExitCode::FAILURE;
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
        match cynic_storage::evidence_journal_file::EvidenceJournalStore::open(evidence_root) {
            Ok(store) => {
                let payload_json = serde_json::to_string(&report).unwrap_or_default();
                let entry = cynic_protocol::evidence_journal::EvidenceJournalEntry::new(
                    cynic_protocol::tabular::TABULAR_VALIDATION_REPORT_SCHEMA_V1,
                    payload_json.as_bytes(),
                    "agent:antigravity:data_validation",
                    "validation_run",
                );
                if let Err(e) = store.append(&entry) {
                    eprintln!("Warning: Failed to seal evidence journal entry: {}", e);
                } else if !json {
                    println!("Sealed Evidence: journal append OK (hash: {:.16}..)", entry.entry_hash);
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
                    DriftSeverity::Info => "INFO",
                };
                match &drift.drift {
                    SchemaDriftKind::MissingColumn { column } => {
                        println!("  - [{}] [{}] Missing required column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::UnexpectedColumn { column } => {
                        println!("  - [{}] [{}] Unexpected column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::TypeMismatch { column, expected, observed } => {
                        println!(
                            "  - [{}] [{}] Type mismatch for '{}': expected {}, observed '{}'",
                            sev, row_str, column, expected, observed
                        );
                    }
                    SchemaDriftKind::MissingRequiredValue { column } => {
                        println!("  - [{}] [{}] Column '{}' is null or empty but marked required", sev, row_str, column);
                    }
                    SchemaDriftKind::MalformedRecord { message } => {
                        println!("  - [{}] [{}] Malformed record: {}", sev, row_str, message);
                    }
                    SchemaDriftKind::RangeViolation { column, value, min, max } => {
                        println!(
                            "  - [{}] [{}] Range violation for '{}': value {} outside [{:?}, {:?}]",
                            sev, row_str, column, value, min, max
                        );
                    }
                    SchemaDriftKind::AllowedValuesViolation { column, value, allowed } => {
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
