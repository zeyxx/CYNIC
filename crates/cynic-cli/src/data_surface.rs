//! Data domain CLI surface: schema validation, drift detection, and contract templates.

use clap::Subcommand;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use cynic_protocol::tabular::{
    ColumnDefinition, DataColumnType, DriftSeverity, SchemaDriftKind, TabularSchema,
};
use cynic_traces::tabular::validate_tabular_file;

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
    },
    /// Emit a canonical TabularSchema contract for reference
    Schema {
        /// Schema kind: 'asset' (IT Asset Management) or 'claim' (NatCat Claims)
        #[arg(long, default_value = "asset")]
        kind: String,
    },
}

pub fn execute_data(command: DataCommand) -> ExitCode {
    match command {
        DataCommand::Validate { schema, file, json } => execute_validate(&schema, &file, json),
        DataCommand::Schema { kind } => execute_schema(&kind),
    }
}

fn execute_validate(schema_path: &Path, file_path: &Path, json: bool) -> ExitCode {
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

    let report = match validate_tabular_file(&schema, file_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: Validation failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

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
        "claim" | "claims" | "natcat" => TabularSchema::new(
            "cynic.natcat_claims.v1",
            1,
            vec![
                ColumnDefinition::new("claim_id", DataColumnType::Text, true)
                    .with_description("Unique identifier of the insurance claim"),
                ColumnDefinition::new("event_type", DataColumnType::Text, true)
                    .with_description("Peril type: FLOOD, STORM, DROUGHT, EARTHQUAKE"),
                ColumnDefinition::new("loss_amount", DataColumnType::Float, true)
                    .with_description("Estimated or settled loss in EUR"),
                ColumnDefinition::new("settled", DataColumnType::Boolean, true)
                    .with_description("Settlement flag"),
                ColumnDefinition::new("event_date", DataColumnType::TimestampIso, true)
                    .with_description("Date or timestamp of the catastrophe"),
            ],
        )
        .with_strict_columns(true),
        _ => TabularSchema::new(
            "cynic.it_asset.v1",
            1,
            vec![
                ColumnDefinition::new("asset_id", DataColumnType::Text, true)
                    .with_description("Unique asset identifier"),
                ColumnDefinition::new("hostname", DataColumnType::Text, true)
                    .with_description("FQDN of the asset"),
                ColumnDefinition::new("ip_address", DataColumnType::Text, false)
                    .with_description("Primary IP address"),
                ColumnDefinition::new("criticality", DataColumnType::Integer, true)
                    .with_description("Business impact score (1 to 5)"),
                ColumnDefinition::new("compliant", DataColumnType::Boolean, true)
                    .with_description("SSI compliance status"),
                ColumnDefinition::new("last_audit", DataColumnType::TimestampIso, false)
                    .with_description("Last security audit scan timestamp"),
            ],
        )
        .with_strict_columns(true),
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
