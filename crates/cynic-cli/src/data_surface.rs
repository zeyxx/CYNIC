//! Data domain CLI surface: schema validation, drift detection, and contract templates.

use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_protocol::tabular::{
    DriftSeverity, SchemaDriftKind, TabularSchema, TabularValidationReport,
};

#[derive(Subcommand, Debug, Clone)]
pub enum DataCommand {
    /// Validate a CSV or JSONL file against a TabularSchema contract
    Validate {
        /// Path to the JSON schema file (TabularSchema)
        #[arg(long, short)]
        schema: PathBuf,
        /// Path to the tabular data file (CSV or JSONL)
        #[arg(long, short)]
        file: PathBuf,
        /// Output validation report as JSON
        #[arg(long)]
        json: bool,
        /// Record validation run in evidence history (evidence/data_validation_history.jsonl)
        #[arg(long)]
        record: bool,
    },
    /// Emit a canonical TabularSchema contract for reference
    Schema {
        /// Schema kind: 'asset' (IT Asset Management) or 'claim' (NatCat Claims)
        #[arg(long, default_value = "asset")]
        kind: String,
    },
    /// View historical tabular validation runs from evidence history
    History {
        /// Limit number of historical records to display
        #[arg(long, default_value_t = 10)]
        limit: usize,
        /// Output history as JSON
        #[arg(long)]
        json: bool,
    },
}

pub fn execute_data(command: DataCommand) -> ExitCode {
    match command {
        DataCommand::Validate { schema, file, json, record } => execute_validate(&schema, &file, json, record),
        DataCommand::Schema { kind } => execute_schema(&kind),
        DataCommand::History { limit, json } => execute_history(limit, json),
    }
}

fn execute_validate(schema_path: &Path, file_path: &Path, json: bool, record: bool) -> ExitCode {
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
    }

    if json {
        match serde_json::to_string_pretty(&report) {
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
        println!(
            "Status        : {}",
            if report.passed { "PASSED" } else { "FAILED" }
        );

        if !report.drifts.is_empty() {
            println!();
            println!("Detected Drifts / Violations ({} total):", report.drifts.len());
            for drift in &report.drifts {
                let sev = match drift.severity {
                    DriftSeverity::Critical => "CRITICAL",
                    DriftSeverity::Warning => "WARN",
                };
                let row_str = drift
                    .row_index
                    .map(|r| format!("Row {}", r))
                    .unwrap_or_else(|| "Header".to_string());

                match &drift.drift {
                    SchemaDriftKind::ColumnMissing { column } => {
                        println!("  - [{}] [{}] Missing required column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::UnexpectedColumn { column } => {
                        println!("  - [{}] [{}] Unexpected column: '{}'", sev, row_str, column);
                    }
                    SchemaDriftKind::TypeMismatch { column, expected, found_sample } => {
                        println!(
                            "  - [{}] [{}] Column '{}' type mismatch: expected '{}', found '{}'",
                            sev, row_str, column, expected, found_sample
                        );
                    }
                    SchemaDriftKind::NullViolation { column } => {
                        println!("  - [{}] [{}] Column '{}' is null or empty but marked required", sev, row_str, column);
                    }
                    SchemaDriftKind::MalformedRecord { message } => {
                        println!("  - [{}] [{}] Malformed record: {}", sev, row_str, message);
                    }
                    SchemaDriftKind::ValueNotAllowed { column, value, allowed } => {
                        println!(
                            "  - [{}] [{}] Column '{}' value '{}' not in allowed set: [{}]",
                            sev, row_str, column, value, allowed.join(", ")
                        );
                    }
                    SchemaDriftKind::ValueOutOfRange { column, value, min, max } => {
                        let range_desc = match (min, max) {
                            (Some(min_v), Some(max_v)) => format!("[{}..{}]", min_v, max_v),
                            (Some(min_v), None) => format!("[{}..]", min_v),
                            (None, Some(max_v)) => format!("[..{}]", max_v),
                            (None, None) => "unbounded".to_string(),
                        };
                        println!(
                            "  - [{}] [{}] Column '{}' value {} is out of range {}",
                            sev, row_str, column, value, range_desc
                        );
                    }
                    SchemaDriftKind::PatternMismatch { column, value, pattern } => {
                        println!(
                            "  - [{}] [{}] Column '{}' value '{}' does not match pattern '{}'",
                            sev, row_str, column, value, pattern
                        );
                    }
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

fn execute_schema(kind: &str) -> ExitCode {
    let schema = match kind {
        "node" | "nodes" | "registry" | "inventory" => cynic_traces::tabular::node_inventory_schema(),
        "claim" | "claims" | "natcat" => cynic_traces::tabular::natcat_claims_schema(),
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

fn append_history_report(report: &TabularValidationReport, path: &Path) -> Result<(), String> {
    let json_line = serde_json::to_string(report)
        .map_err(|e| format!("Failed to serialize TabularValidationReport: {}", e))?;
    cynic_host::adapters::jsonl_append::JsonlAppend::append_with_fsync(path, &json_line)
}

fn read_history_reports(path: &Path) -> Result<Vec<TabularValidationReport>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open tabular history at {:?}: {}", path, e))?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let mut reports = Vec::new();
    for (idx, line_res) in reader.lines().enumerate() {
        let line = line_res
            .map_err(|e| format!("Failed to read line {} from {:?}: {}", idx + 1, path, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let rep = serde_json::from_str::<TabularValidationReport>(trimmed)
            .map_err(|e| format!("Invalid JSON on line {} in {:?}: {}", idx + 1, path, e))?;
        reports.push(rep);
    }
    Ok(reports)
}

fn execute_history(limit: usize, json: bool) -> ExitCode {
    let history_path = Path::new("evidence/data_validation_history.jsonl");
    let reports = match read_history_reports(history_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading tabular history from '{}': {}", history_path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    if json {
        let slice: Vec<_> = reports.iter().rev().take(limit).collect();
        match serde_json::to_string_pretty(&slice) {
            Ok(s) => {
                println!("{}", s);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("Error serializing history JSON: {}", e);
                ExitCode::FAILURE
            }
        }
    } else {
        println!("=== CYNIC Tabular Validation History ===");
        println!("History Log   : {}", history_path.display());
        println!("Total Runs    : {}", reports.len());
        println!("Display Limit : {}", limit);
        println!();

        if reports.is_empty() {
            println!("No validation runs recorded yet in {}", history_path.display());
        } else {
            for (idx, report) in reports.iter().rev().take(limit).enumerate() {
                let status_str = if report.passed { "PASSED" } else { "FAILED" };
                println!(
                    "#{:<3} [{}] Schema: {} (v{}) | Total Rows: {} | Valid: {} | Invalid: {} | Drifts: {}",
                    idx + 1,
                    status_str,
                    report.target_schema_id,
                    report.target_schema_version,
                    report.total_rows,
                    report.valid_rows,
                    report.invalid_rows,
                    report.drifts.len()
                );
            }
        }
        ExitCode::SUCCESS
    }
}
