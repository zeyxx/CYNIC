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

    let report = match validate_tabular_file(&schema, file_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: Validation failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if record {
        let history_path = Path::new("evidence/data_validation_history.jsonl");
        if let Err(e) = cynic_traces::tabular::append_tabular_report(&report, history_path) {
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
        "node" | "nodes" | "registry" | "inventory" => TabularSchema::new(
            "cynic.node_inventory.v1",
            1,
            vec![
                ColumnDefinition::new("id", DataColumnType::Text, true)
                    .with_description("Unique identifier of the node (hostname)")
                    .with_pattern(r"^[a-z0-9-]+$"),
                ColumnDefinition::new("name", DataColumnType::Text, true)
                    .with_description("Display name of the node"),
                ColumnDefinition::new("role", DataColumnType::Text, false)
                    .with_description("Node architectural role (hypervisor, cortex_host, builder, etc.)"),
                ColumnDefinition::new("status", DataColumnType::Text, true)
                    .with_description("Observed or candidate status")
                    .with_allowed_values(vec![
                        "observed".into(),
                        "candidate".into(),
                        "stub".into(),
                        "decommissioned".into(),
                    ]),
                ColumnDefinition::new("deploy_target", DataColumnType::Boolean, true)
                    .with_description("Whether this node accepts direct workload deployment"),
            ],
        )
        .with_strict_columns(false),
        "claim" | "claims" | "natcat" => TabularSchema::new(
            "cynic.natcat_claims.v1",
            1,
            vec![
                ColumnDefinition::new("claim_id", DataColumnType::Text, true)
                    .with_description("Unique identifier of the insurance claim")
                    .with_pattern(r"^CLM-[0-9]{4}-[0-9]{3,}$"),
                ColumnDefinition::new("event_type", DataColumnType::Text, true)
                    .with_description("Peril type: FLOOD, STORM, DROUGHT, EARTHQUAKE, WILDFIRE")
                    .with_allowed_values(vec![
                        "FLOOD".into(),
                        "STORM".into(),
                        "DROUGHT".into(),
                        "EARTHQUAKE".into(),
                        "WILDFIRE".into(),
                    ]),
                ColumnDefinition::new("loss_amount", DataColumnType::Float, true)
                    .with_description("Estimated or settled loss in EUR")
                    .with_range(Some(0.0), None),
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
                    .with_description("Unique asset identifier")
                    .with_pattern(r"^AST-[0-9]{4,}$"),
                ColumnDefinition::new("hostname", DataColumnType::Text, true)
                    .with_description("FQDN of the asset"),
                ColumnDefinition::new("ip_address", DataColumnType::Text, false)
                    .with_description("Primary IP address"),
                ColumnDefinition::new("criticality", DataColumnType::Integer, true)
                    .with_description("Business impact score (1 to 5)")
                    .with_range(Some(1.0), Some(5.0)),
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

fn execute_history(limit: usize, json: bool) -> ExitCode {
    let history_path = Path::new("evidence/data_validation_history.jsonl");
    let reports = match cynic_traces::tabular::read_tabular_reports(history_path) {
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
