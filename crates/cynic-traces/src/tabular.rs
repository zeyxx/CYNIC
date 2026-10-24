//! Tabular data parser, validator, and drift engine.
//!
//! Provides zero-I/O tabular validation for CSV and JSONL records against
//! canonical `TabularSchema` definitions from `cynic-protocol`.
//! Generates `TabularValidationReport` implementing `DataReport`.

use std::borrow::Cow;
use std::collections::HashSet;
use chrono::DateTime;
use cynic_protocol::tabular::{
    ColumnDefinition, DataColumnType, DriftSeverity, SchemaDrift, SchemaDriftKind,
    TabularSchema, TabularValidationReport,
};

use crate::session::NormalizedSession;

/// Pure RFC-4180 compliant CSV parser for zero-I/O validation.
pub fn parse_csv_records(input: &str) -> Result<(Vec<String>, Vec<Vec<String>>), String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut current_row: Vec<String> = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped quote
                    current_field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else {
            match c {
                '"' => {
                    in_quotes = true;
                }
                ',' => {
                    current_row.push(current_field.trim().to_string());
                    current_field = String::new();
                }
                '\r' => {
                    // Skip carriage return if followed by newline
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    current_row.push(current_field.trim().to_string());
                    current_field = String::new();
                    if !current_row.is_empty() && (current_row.len() > 1 || !current_row[0].is_empty()) {
                        rows.push(current_row);
                    }
                    current_row = Vec::new();
                }
                '\n' => {
                    current_row.push(current_field.trim().to_string());
                    current_field = String::new();
                    if !current_row.is_empty() && (current_row.len() > 1 || !current_row[0].is_empty()) {
                        rows.push(current_row);
                    }
                    current_row = Vec::new();
                }
                _ => {
                    current_field.push(c);
                }
            }
        }
    }

    if in_quotes {
        return Err("Unclosed quote detected in CSV content".to_string());
    }

    if !current_field.is_empty() || !current_row.is_empty() {
        current_row.push(current_field.trim().to_string());
        if current_row.len() > 1 || !current_row[0].is_empty() {
            rows.push(current_row);
        }
    }

    if rows.is_empty() {
        return Err("CSV content is empty".to_string());
    }

    let header = rows.remove(0);
    Ok((header, rows))
}

/// Pure JSONL parser returning a list of JSON objects.
pub fn parse_jsonl_records(input: &str) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, String> {
    let mut records = Vec::new();
    for (line_idx, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(serde_json::Value::Object(map)) => records.push(map),
            Ok(_) => {
                return Err(format!(
                    "Line {}: JSONL record must be a JSON object",
                    line_idx + 1
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Line {}: Malformed JSON record: {}",
                    line_idx + 1,
                    e
                ));
            }
        }
    }
    Ok(records)
}

/// Check if a string representation matches the expected column type.
pub fn validate_scalar_value(
    data_type: DataColumnType,
    val_str: &str,
) -> Result<(), String> {
    match data_type {
        DataColumnType::Text => Ok(()),
        DataColumnType::Integer => {
            val_str
                .parse::<i64>()
                .map(|_| ())
                .map_err(|_| format!("Cannot parse '{}' as integer", val_str))
        }
        DataColumnType::Float => {
            val_str
                .parse::<f64>()
                .map(|_| ())
                .map_err(|_| format!("Cannot parse '{}' as float", val_str))
        }
        DataColumnType::Boolean => {
            match val_str.to_ascii_lowercase().as_str() {
                "true" | "false" | "1" | "0" | "t" | "f" | "yes" | "no" => Ok(()),
                _ => Err(format!("Cannot parse '{}' as boolean", val_str)),
            }
        }
        DataColumnType::TimestampIso => {
            if DateTime::parse_from_rfc3339(val_str).is_ok() {
                Ok(())
            } else if chrono::NaiveDateTime::parse_from_str(val_str, "%Y-%m-%dT%H:%M:%S").is_ok() {
                Ok(())
            } else if chrono::NaiveDate::parse_from_str(val_str, "%Y-%m-%d").is_ok() {
                Ok(())
            } else {
                Err(format!("Cannot parse '{}' as ISO timestamp", val_str))
            }
        }
        DataColumnType::Json => {
            serde_json::from_str::<serde_json::Value>(val_str)
                .map(|_| ())
                .map_err(|e| format!("Invalid JSON snippet: {}", e))
        }
    }
}

/// Pre-compiled column definition caching parsed regex and constraints for O(1) cell evaluation.
#[derive(Debug, Clone)]
pub struct CompiledColumn {
    pub def: ColumnDefinition,
    pub compiled_regex: Option<regex::Regex>,
}

impl CompiledColumn {
    pub fn new(def: ColumnDefinition) -> Result<Self, String> {
        let compiled_regex = match &def.pattern {
            Some(pat) => Some(
                regex::Regex::new(pat)
                    .map_err(|e| format!("Invalid regex pattern '{}' for column '{}': {}", pat, def.name, e))?,
            ),
            None => None,
        };
        Ok(Self { def, compiled_regex })
    }

    /// Pure constraint validation without recompiling regexes.
    pub fn validate_constraints(&self, val_str: &str) -> Vec<SchemaDriftKind> {
        let mut drifts = Vec::new();

        // 1. Allowed values constraint
        if let Some(allowed) = &self.def.allowed_values {
            if !allowed.iter().any(|v| v.as_str() == val_str) {
                drifts.push(SchemaDriftKind::ValueNotAllowed {
                    column: self.def.name.clone(),
                    value: val_str.to_string(),
                    allowed: allowed.clone(),
                });
            }
        }

        // 2. Numeric range constraint (min / max)
        if self.def.min_value.is_some() || self.def.max_value.is_some() {
            if let Ok(num) = val_str.parse::<f64>() {
                let out_of_min = self.def.min_value.map_or(false, |min| num < min);
                let out_of_max = self.def.max_value.map_or(false, |max| num > max);
                if out_of_min || out_of_max {
                    drifts.push(SchemaDriftKind::ValueOutOfRange {
                        column: self.def.name.clone(),
                        value: num,
                        min: self.def.min_value,
                        max: self.def.max_value,
                    });
                }
            }
        }

        // 3. Regex pattern constraint (using precompiled regex)
        if let Some(re) = &self.compiled_regex {
            if !re.is_match(val_str) {
                drifts.push(SchemaDriftKind::PatternMismatch {
                    column: self.def.name.clone(),
                    value: val_str.to_string(),
                    pattern: self.def.pattern.clone().unwrap_or_default(),
                });
            }
        }

        drifts
    }
}

/// Compiled schema caching pre-parsed constraints for ultra-fast validation over large row sets.
#[derive(Debug, Clone)]
pub struct CompiledTabularSchema {
    pub schema: TabularSchema,
    pub columns: Vec<CompiledColumn>,
}

impl CompiledTabularSchema {
    pub fn compile(schema: &TabularSchema) -> Result<Self, String> {
        let mut columns = Vec::with_capacity(schema.columns.len());
        for col in &schema.columns {
            columns.push(CompiledColumn::new(col.clone())?);
        }
        Ok(Self {
            schema: schema.clone(),
            columns,
        })
    }

    pub fn get_column(&self, name: &str) -> Option<&CompiledColumn> {
        self.columns.iter().find(|c| c.def.name == name)
    }
}

/// Backward-compatible constraint check delegating to CompiledColumn.
pub fn validate_column_constraints(
    col_def: &ColumnDefinition,
    val_str: &str,
) -> Vec<SchemaDriftKind> {
    match CompiledColumn::new(col_def.clone()) {
        Ok(compiled) => compiled.validate_constraints(val_str),
        Err(e) => vec![SchemaDriftKind::MalformedRecord { message: e }],
    }
}

/// Validate CSV records against a TabularSchema.
pub fn validate_csv_content(schema: &TabularSchema, csv_str: &str) -> TabularValidationReport {
    let compiled = match CompiledTabularSchema::compile(schema) {
        Ok(c) => c,
        Err(e) => {
            let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord { message: e },
            });
            report.passed = false;
            return report;
        }
    };
    validate_csv_with_compiled(&compiled, csv_str)
}

/// Validate CSV records using an already compiled TabularSchema (zero-recompilation).
pub fn validate_csv_with_compiled(
    compiled: &CompiledTabularSchema,
    csv_str: &str,
) -> TabularValidationReport {
    let schema = &compiled.schema;
    let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);

    let (headers, rows) = match parse_csv_records(csv_str) {
        Ok(parsed) => parsed,
        Err(e) => {
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord { message: e },
            });
            report.passed = false;
            return report;
        }
    };

    let header_set: HashSet<&str> = headers.iter().map(|s| s.as_str()).collect();

    // 1. Check for missing required columns (dataset-level)
    for col in &compiled.columns {
        if col.def.required && !header_set.contains(col.def.name.as_str()) {
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::ColumnMissing {
                    column: col.def.name.clone(),
                },
            });
        }
    }

    // 2. Check for unexpected columns in strict mode
    if schema.strict_columns {
        for header in &headers {
            if compiled.get_column(header).is_none() {
                report.record_drift(SchemaDrift {
                    row_index: None,
                    severity: DriftSeverity::Warning,
                    drift: SchemaDriftKind::UnexpectedColumn {
                        column: header.clone(),
                    },
                });
            }
        }
    }

    // Map column index to compiled column definition
    let col_mappings: Vec<Option<&CompiledColumn>> = headers
        .iter()
        .map(|h| compiled.get_column(h))
        .collect();

    report.total_rows = rows.len();

    // 3. Row-by-row validation
    for (row_idx, row) in rows.iter().enumerate() {
        let mut row_has_error = false;
        let actual_row_num = row_idx + 1;

        if row.len() != headers.len() {
            report.record_drift(SchemaDrift {
                row_index: Some(actual_row_num),
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord {
                    message: format!(
                        "Column count mismatch: expected {}, found {}",
                        headers.len(),
                        row.len()
                    ),
                },
            });
            row_has_error = true;
        }

        for (col_idx, cell) in row.iter().enumerate() {
            if let Some(Some(compiled_col)) = col_mappings.get(col_idx) {
                if cell.is_empty() {
                    if compiled_col.def.required {
                        report.record_drift(SchemaDrift {
                            row_index: Some(actual_row_num),
                            severity: DriftSeverity::Critical,
                            drift: SchemaDriftKind::NullViolation {
                                column: compiled_col.def.name.clone(),
                            },
                        });
                        row_has_error = true;
                    }
                } else if let Err(_) = validate_scalar_value(compiled_col.def.data_type, cell) {
                    report.record_drift(SchemaDrift {
                        row_index: Some(actual_row_num),
                        severity: DriftSeverity::Critical,
                        drift: SchemaDriftKind::TypeMismatch {
                            column: compiled_col.def.name.clone(),
                            expected: compiled_col.def.data_type.to_string(),
                            found_sample: cell.clone(),
                        },
                    });
                    row_has_error = true;
                } else {
                    for drift_kind in compiled_col.validate_constraints(cell) {
                        report.record_drift(SchemaDrift {
                            row_index: Some(actual_row_num),
                            severity: DriftSeverity::Critical,
                            drift: drift_kind,
                        });
                        row_has_error = true;
                    }
                }
            }
        }

        if row_has_error {
            report.invalid_rows += 1;
        } else {
            report.valid_rows += 1;
        }
    }

    report.passed = report.invalid_rows == 0
        && !report.drifts.iter().any(|d| d.severity == DriftSeverity::Critical);

    report
}

/// Pure JSON parser extracting records from a JSON array or container object (e.g. registry with "nodes").
pub fn parse_json_records(input: &str) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, String> {
    let parsed: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| format!("Malformed JSON input: {}", e))?;

    let array_opt = match &parsed {
        serde_json::Value::Array(arr) => Some(arr),
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(arr)) = map.get("nodes") {
                Some(arr)
            } else if let Some(serde_json::Value::Array(arr)) = map.get("records") {
                Some(arr)
            } else if let Some(serde_json::Value::Array(arr)) = map.get("items") {
                Some(arr)
            } else if let Some(serde_json::Value::Array(arr)) = map.get("entries") {
                Some(arr)
            } else {
                None
            }
        }
        _ => None,
    };

    match array_opt {
        Some(arr) => {
            let mut records = Vec::new();
            for (idx, item) in arr.iter().enumerate() {
                if let serde_json::Value::Object(map) = item {
                    records.push(map.clone());
                } else {
                    return Err(format!("Item at index {} is not a JSON object", idx));
                }
            }
            Ok(records)
        }
        None => {
            if let serde_json::Value::Object(map) = parsed {
                Ok(vec![map])
            } else {
                Err("JSON root must be an array or an object".to_string())
            }
        }
    }
}

/// Validate parsed record maps against a TabularSchema.
pub fn validate_record_maps(
    schema: &TabularSchema,
    records: &[serde_json::Map<String, serde_json::Value>],
) -> TabularValidationReport {
    let compiled = match CompiledTabularSchema::compile(schema) {
        Ok(c) => c,
        Err(e) => {
            let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord { message: e },
            });
            report.passed = false;
            return report;
        }
    };
    validate_records_with_compiled(&compiled, records)
}

/// Validate parsed record maps using an already compiled TabularSchema (zero-recompilation).
pub fn validate_records_with_compiled(
    compiled: &CompiledTabularSchema,
    records: &[serde_json::Map<String, serde_json::Value>],
) -> TabularValidationReport {
    let schema = &compiled.schema;
    let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);
    report.total_rows = records.len();

    for (row_idx, record) in records.iter().enumerate() {
        let mut row_has_error = false;
        let actual_row_num = row_idx + 1;

        // Check required columns
        for compiled_col in &compiled.columns {
            let col = &compiled_col.def;
            match record.get(&col.name) {
                None => {
                    if col.required {
                        report.record_drift(SchemaDrift {
                            row_index: Some(actual_row_num),
                            severity: DriftSeverity::Critical,
                            drift: SchemaDriftKind::ColumnMissing {
                                column: col.name.clone(),
                            },
                        });
                        row_has_error = true;
                    }
                }
                Some(serde_json::Value::Null) => {
                    if col.required {
                        report.record_drift(SchemaDrift {
                            row_index: Some(actual_row_num),
                            severity: DriftSeverity::Critical,
                            drift: SchemaDriftKind::NullViolation {
                                column: col.name.clone(),
                            },
                        });
                        row_has_error = true;
                    }
                }
                Some(val) => {
                    let type_matches = match col.data_type {
                        DataColumnType::Text => val.is_string(),
                        DataColumnType::Integer => val.is_i64(),
                        DataColumnType::Float => val.is_f64() || val.is_i64(),
                        DataColumnType::Boolean => val.is_boolean(),
                        DataColumnType::TimestampIso => {
                            if let Some(s) = val.as_str() {
                                validate_scalar_value(DataColumnType::TimestampIso, s).is_ok()
                            } else {
                                false
                            }
                        }
                        DataColumnType::Json => true,
                    };

                    if !type_matches {
                        report.record_drift(SchemaDrift {
                            row_index: Some(actual_row_num),
                            severity: DriftSeverity::Critical,
                            drift: SchemaDriftKind::TypeMismatch {
                                column: col.name.clone(),
                                expected: col.data_type.to_string(),
                                found_sample: val.to_string(),
                            },
                        });
                        row_has_error = true;
                    } else {
                        let val_cow = match val {
                            serde_json::Value::String(s) => Cow::Borrowed(s.as_str()),
                            serde_json::Value::Number(n) => Cow::Owned(n.to_string()),
                            serde_json::Value::Bool(b) => Cow::Borrowed(if *b { "true" } else { "false" }),
                            _ => Cow::Owned(val.to_string()),
                        };
                        for drift_kind in compiled_col.validate_constraints(&val_cow) {
                            report.record_drift(SchemaDrift {
                                row_index: Some(actual_row_num),
                                severity: DriftSeverity::Critical,
                                drift: drift_kind,
                            });
                            row_has_error = true;
                        }
                    }
                }
            }
        }

        // Strict mode check
        if schema.strict_columns {
            for key in record.keys() {
                if compiled.get_column(key).is_none() {
                    report.record_drift(SchemaDrift {
                        row_index: Some(actual_row_num),
                        severity: DriftSeverity::Warning,
                        drift: SchemaDriftKind::UnexpectedColumn {
                            column: key.clone(),
                        },
                    });
                }
            }
        }

        if row_has_error {
            report.invalid_rows += 1;
        } else {
            report.valid_rows += 1;
        }
    }

    report.passed = report.invalid_rows == 0
        && !report.drifts.iter().any(|d| d.severity == DriftSeverity::Critical);

    report
}

/// Validate JSONL records against a TabularSchema.
pub fn validate_jsonl_content(schema: &TabularSchema, jsonl_str: &str) -> TabularValidationReport {
    match parse_jsonl_records(jsonl_str) {
        Ok(rec) => validate_record_maps(schema, &rec),
        Err(e) => {
            let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord { message: e },
            });
            report.passed = false;
            report
        }
    }
}

/// Validate JSON records (array or container object) against a TabularSchema.
pub fn validate_json_content(schema: &TabularSchema, json_str: &str) -> TabularValidationReport {
    match parse_json_records(json_str) {
        Ok(rec) => validate_record_maps(schema, &rec),
        Err(e) => {
            let mut report = TabularValidationReport::new(&schema.schema_id, schema.version);
            report.record_drift(SchemaDrift {
                row_index: None,
                severity: DriftSeverity::Critical,
                drift: SchemaDriftKind::MalformedRecord { message: e },
            });
            report.passed = false;
            report
        }
    }
}

/// Canonical schema for node inventory and topology.
pub fn node_inventory_schema() -> TabularSchema {
    TabularSchema::new(
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
    .with_strict_columns(false)
}

/// Canonical schema for catastrophe insurance claims.
pub fn natcat_claims_schema() -> TabularSchema {
    TabularSchema::new(
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
    .with_strict_columns(true)
}

/// Canonical schema for agent session trace summaries.
pub fn session_summary_schema() -> TabularSchema {
    TabularSchema::new(
        "cynic.session_summary.v1",
        1,
        vec![
            ColumnDefinition::new("session_uid", DataColumnType::Text, true)
                .with_description("Unique identifier of the normalized session")
                .with_pattern(r"^[a-zA-Z0-9_\-\.:]+$"),
            ColumnDefinition::new("harness", DataColumnType::Text, true)
                .with_description("Harness originating the trace")
                .with_allowed_values(vec![
                    "hermes".into(),
                    "claude".into(),
                    "codestory".into(),
                    "antigravity".into(),
                    "gemini".into(),
                    "cursor".into(),
                    "copilot".into(),
                    "aider".into(),
                    "openai".into(),
                ]),
            ColumnDefinition::new("source_ref", DataColumnType::Text, true)
                .with_description("Source file or URI of the raw trace"),
            ColumnDefinition::new("turns", DataColumnType::Integer, true)
                .with_description("Total conversation turns")
                .with_range(Some(0.0), None),
            ColumnDefinition::new("tools", DataColumnType::Integer, true)
                .with_description("Total tool executions")
                .with_range(Some(0.0), None),
            ColumnDefinition::new("reasoning", DataColumnType::Integer, true)
                .with_description("Total reasoning trace blocks")
                .with_range(Some(0.0), None),
            ColumnDefinition::new("edges", DataColumnType::Integer, true)
                .with_description("Total causal DAG edges")
                .with_range(Some(0.0), None),
            ColumnDefinition::new("status", DataColumnType::Text, true)
                .with_description("Invariant validation status")
                .with_allowed_values(vec![
                    "ok".into(),
                    "failed".into(),
                    "partial".into(),
                    "contract_violation".into(),
                ]),
        ],
    )
    .with_strict_columns(false)
}

/// Canonical schema for individual conversation turns in a session.
pub fn session_turn_schema() -> TabularSchema {
    TabularSchema::new(
        "cynic.session_turn.v1",
        1,
        vec![
            ColumnDefinition::new("turn_uid", DataColumnType::Text, true)
                .with_description("Unique identifier of the conversation turn")
                .with_pattern(r"^[a-zA-Z0-9_\-\.:]+$"),
            ColumnDefinition::new("session_uid", DataColumnType::Text, true)
                .with_description("Reference to the parent session UID")
                .with_pattern(r"^[a-zA-Z0-9_\-\.:]+$"),
            ColumnDefinition::new("index", DataColumnType::Integer, true)
                .with_description("Zero-based sequence index within the session")
                .with_range(Some(0.0), None),
            ColumnDefinition::new("role", DataColumnType::Text, true)
                .with_description("Speaker role")
                .with_allowed_values(vec![
                    "user".into(),
                    "assistant".into(),
                    "system".into(),
                    "tool".into(),
                ]),
            ColumnDefinition::new("native_model", DataColumnType::Text, false)
                .with_description("Raw model identifier as reported by the harness"),
        ],
    )
    .with_strict_columns(false)
}

/// Canonical schema for document chunks ingested into BM25 / RAG vector storage.
pub fn document_chunk_schema() -> TabularSchema {
    TabularSchema::new(
        "cynic.document_chunk.v1",
        1,
        vec![
            ColumnDefinition::new("chunk_id", DataColumnType::Text, true)
                .with_description("Deterministic BLAKE3 hex hash of chunk content and location")
                .with_pattern(r"^[a-f0-9]{64}$"),
            ColumnDefinition::new("source_path", DataColumnType::Text, true)
                .with_description("Repository-relative source document path"),
            ColumnDefinition::new("line_start", DataColumnType::Integer, true)
                .with_description("1-indexed starting line number")
                .with_range(Some(1.0), None),
            ColumnDefinition::new("line_end", DataColumnType::Integer, true)
                .with_description("1-indexed ending line number")
                .with_range(Some(1.0), None),
            ColumnDefinition::new("char_length", DataColumnType::Integer, true)
                .with_description("Character length of chunk content (semantic starvation vs overflow)")
                .with_range(Some(60.0), Some(8000.0)),
            ColumnDefinition::new("authority", DataColumnType::Text, true)
                .with_description("Constitutional ranking tier for epistemic retrieval")
                .with_allowed_values(vec![
                    "L0Constitution".into(),
                    "L1Specification".into(),
                    "L2Design".into(),
                    "L3Operation".into(),
                ]),
        ],
    )
    .with_strict_columns(false)
}

/// Canonical schema for verifiable facts and epistemic claims.
pub fn epistemic_claim_schema() -> TabularSchema {
    TabularSchema::new(
        "cynic.epistemic_claim.v1",
        1,
        vec![
            ColumnDefinition::new("id", DataColumnType::Text, true)
                .with_description("Unique identifier of the verifiable claim")
                .with_pattern(r"^[a-zA-Z0-9_\-\.:]+$"),
            ColumnDefinition::new("category", DataColumnType::Text, true)
                .with_description("Domain category of the claim")
                .with_allowed_values(vec![
                    "architecture".into(),
                    "code_volume".into(),
                    "hygiene".into(),
                    "provenance".into(),
                    "competence".into(),
                    "trajectory".into(),
                    "event".into(),
                ]),
            ColumnDefinition::new("statement", DataColumnType::Text, true)
                .with_description("Concise factual statement asserting an empirical reality"),
            ColumnDefinition::new("status", DataColumnType::Text, true)
                .with_description("Audit verdict backed by machine receipt")
                .with_allowed_values(vec![
                    "demonstrated".into(),
                    "implemented".into(),
                    "partial".into(),
                    "aspirational".into(),
                    "unknown".into(),
                ]),
            ColumnDefinition::new("evidence", DataColumnType::Text, true)
                .with_description("Path to receipt, journal entry, or reproducible command"),
            ColumnDefinition::new("verify_command", DataColumnType::Text, false)
                .with_description("Exact command to execute to re-verify claim on physical substrate"),
        ],
    )
    .with_strict_columns(false)
}

/// Projects a `NormalizedSession` into a tabular row matching `session_summary_schema()`.
pub fn session_to_summary_record(session: &NormalizedSession) -> serde_json::Map<String, serde_json::Value> {
    let stats = session.stats();
    let mut map = serde_json::Map::new();
    map.insert("session_uid".to_string(), serde_json::Value::String(session.session_uid.clone()));
    map.insert("harness".to_string(), serde_json::Value::String(session.harness.clone()));
    map.insert("source_ref".to_string(), serde_json::Value::String(session.source_ref.clone()));
    map.insert("turns".to_string(), serde_json::json!(stats.turns as i64));
    map.insert("tools".to_string(), serde_json::json!(stats.tools as i64));
    map.insert("reasoning".to_string(), serde_json::json!(stats.reasoning as i64));
    map.insert("edges".to_string(), serde_json::json!(stats.edges as i64));
    map.insert(
        "status".to_string(),
        serde_json::Value::String(if session.check().is_ok() {
            "ok".to_string()
        } else {
            "contract_violation".to_string()
        }),
    );
    map
}

/// Validates a slice of `NormalizedSession` against a tabular schema.
pub fn validate_sessions(schema: &TabularSchema, sessions: &[NormalizedSession]) -> TabularValidationReport {
    let records: Vec<serde_json::Map<String, serde_json::Value>> = sessions
        .iter()
        .map(session_to_summary_record)
        .collect();
    validate_record_maps(schema, &records)
}

/// Projects raw chunk fields into a `cynic.document_chunk.v1` record map.
pub fn document_chunk_record(
    chunk_id: &str,
    source_path: &str,
    line_start: usize,
    line_end: usize,
    char_length: usize,
    authority: &str,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("chunk_id".to_string(), serde_json::Value::String(chunk_id.to_string()));
    map.insert("source_path".to_string(), serde_json::Value::String(source_path.to_string()));
    map.insert("line_start".to_string(), serde_json::json!(line_start as i64));
    map.insert("line_end".to_string(), serde_json::json!(line_end as i64));
    map.insert("char_length".to_string(), serde_json::json!(char_length as i64));
    map.insert("authority".to_string(), serde_json::Value::String(authority.to_string()));
    map
}

/// Validates an array of document chunk record maps against `document_chunk_schema()`.
pub fn validate_document_chunk_records(
    records: &[serde_json::Map<String, serde_json::Value>],
) -> TabularValidationReport {
    let schema = document_chunk_schema();
    validate_record_maps(&schema, records)
}

/// Projects raw claim fields into a `cynic.epistemic_claim.v1` record map.
pub fn epistemic_claim_record(
    id: &str,
    category: &str,
    statement: &str,
    status: &str,
    evidence: &str,
    verify_command: Option<&str>,
) -> serde_json::Map<String, serde_json::Value> {
    let mut map = serde_json::Map::new();
    map.insert("id".to_string(), serde_json::Value::String(id.to_string()));
    map.insert("category".to_string(), serde_json::Value::String(category.to_string()));
    map.insert("statement".to_string(), serde_json::Value::String(statement.to_string()));
    map.insert("status".to_string(), serde_json::Value::String(status.to_string()));
    map.insert("evidence".to_string(), serde_json::Value::String(evidence.to_string()));
    if let Some(cmd) = verify_command {
        map.insert("verify_command".to_string(), serde_json::Value::String(cmd.to_string()));
    }
    map
}

/// Validates an array of epistemic claim record maps against `epistemic_claim_schema()`.
pub fn validate_epistemic_claim_records(
    records: &[serde_json::Map<String, serde_json::Value>],
) -> TabularValidationReport {
    let schema = epistemic_claim_schema();
    validate_record_maps(&schema, records)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::tabular::ColumnDefinition;

    fn sample_asset_schema() -> TabularSchema {
        TabularSchema::new(
            "cynic.it_asset.v1",
            1,
            vec![
                ColumnDefinition::new("asset_id", DataColumnType::Text, true),
                ColumnDefinition::new("hostname", DataColumnType::Text, true),
                ColumnDefinition::new("criticality", DataColumnType::Integer, true),
                ColumnDefinition::new("is_active", DataColumnType::Boolean, true),
                ColumnDefinition::new("last_audit", DataColumnType::TimestampIso, false),
            ],
        )
        .with_strict_columns(true)
    }

    #[test]
    fn test_valid_csv_validation() {
        let schema = sample_asset_schema();
        let csv = "\
asset_id,hostname,criticality,is_active,last_audit
SRV-001,srv-prod-01.corp,5,true,2026-10-24T10:00:00Z
SRV-002,srv-db-01.corp,4,false,2026-10-23
";
        let report = validate_csv_content(&schema, csv);
        assert!(report.passed);
        assert_eq!(report.total_rows, 2);
        assert_eq!(report.valid_rows, 2);
        assert_eq!(report.invalid_rows, 0);
        assert!(report.drifts.is_empty());
    }

    #[test]
    fn test_csv_missing_column_and_type_mismatch() {
        let schema = sample_asset_schema();
        // Missing "is_active", criticality has invalid integer
        let csv = "\
asset_id,hostname,criticality,last_audit
SRV-001,srv-prod-01.corp,NOT_AN_INT,2026-10-24T10:00:00Z
";
        let report = validate_csv_content(&schema, csv);
        assert!(!report.passed);
        assert_eq!(report.total_rows, 1);
        assert_eq!(report.invalid_rows, 1);
        assert!(report.drifts.iter().any(|d| matches!(
            &d.drift,
            SchemaDriftKind::ColumnMissing { column } if column == "is_active"
        )));
        assert!(report.drifts.iter().any(|d| matches!(
            &d.drift,
            SchemaDriftKind::TypeMismatch { column, .. } if column == "criticality"
        )));
    }

    #[test]
    fn test_csv_strict_unexpected_column() {
        let schema = sample_asset_schema();
        let csv = "\
asset_id,hostname,criticality,is_active,last_audit,rogue_field
SRV-001,srv-prod-01.corp,5,true,2026-10-24T10:00:00Z,malicious_payload
";
        let report = validate_csv_content(&schema, csv);
        // UnexpectedColumn has Warning severity, so valid_rows is 1, passed is true unless Critical
        assert!(report.passed);
        assert!(report.drifts.iter().any(|d| matches!(
            &d.drift,
            SchemaDriftKind::UnexpectedColumn { column } if column == "rogue_field"
        )));
    }

    #[test]
    fn test_jsonl_validation_roundtrip() {
        let schema = sample_asset_schema();
        let jsonl = r#"
{"asset_id": "SRV-001", "hostname": "srv-prod-01.corp", "criticality": 5, "is_active": true, "last_audit": "2026-10-24T10:00:00Z"}
{"asset_id": "SRV-002", "hostname": "srv-db-01.corp", "criticality": "WRONG_TYPE", "is_active": false}
"#;
        let report = validate_jsonl_content(&schema, jsonl);
        assert!(!report.passed);
        assert_eq!(report.total_rows, 2);
        assert_eq!(report.valid_rows, 1);
        assert_eq!(report.invalid_rows, 1);
        assert_eq!(report.drifts.len(), 1);
    }

    #[test]
    fn test_natcat_claims_schema_validation() {
        let natcat_schema = TabularSchema::new(
            "cynic.natcat_claims.v1",
            1,
            vec![
                ColumnDefinition::new("claim_id", DataColumnType::Text, true),
                ColumnDefinition::new("event_type", DataColumnType::Text, true),
                ColumnDefinition::new("loss_amount", DataColumnType::Float, true),
                ColumnDefinition::new("settled", DataColumnType::Boolean, true),
                ColumnDefinition::new("event_date", DataColumnType::TimestampIso, true),
            ],
        );

        let csv = "\
claim_id,event_type,loss_amount,settled,event_date
CLM-2026-001,FLOOD,125000.50,true,2026-05-12
CLM-2026-002,STORM,48200.00,false,2026-08-01T14:30:00Z
";
        let report = validate_csv_content(&natcat_schema, csv);
        assert!(report.passed);
        assert_eq!(report.total_rows, 2);
        assert_eq!(report.valid_rows, 2);
    }

    #[test]
    fn test_json_registry_container_validation() {
        let node_schema = TabularSchema::new(
            "cynic.node_inventory.v1",
            1,
            vec![
                ColumnDefinition::new("id", DataColumnType::Text, true),
                ColumnDefinition::new("role", DataColumnType::Text, true),
                ColumnDefinition::new("deploy_target", DataColumnType::Boolean, true),
            ],
        );

        let registry_json = r#"{
            "schema": "cynic.registry.v1.minimal",
            "nodes": [
                {"id": "node-1", "role": "builder", "deploy_target": true},
                {"id": "node-2", "role": "auditor", "deploy_target": false}
            ]
        }"#;

        let report = validate_json_content(&node_schema, registry_json);
        assert!(report.passed);
        assert_eq!(report.total_rows, 2);
        assert_eq!(report.valid_rows, 2);
        assert_eq!(report.invalid_rows, 0);
    }

    #[test]
    fn test_allowed_values_constraint() {
        let schema = TabularSchema::new(
            "cynic.enum_test.v1",
            1,
            vec![
                ColumnDefinition::new("status", DataColumnType::Text, true)
                    .with_allowed_values(vec!["ACTIVE".into(), "RETIRED".into()]),
            ],
        );

        let csv_valid = "status\nACTIVE\nRETIRED\n";
        let rep_valid = validate_csv_content(&schema, csv_valid);
        assert!(rep_valid.passed);
        assert_eq!(rep_valid.valid_rows, 2);

        let csv_invalid = "status\nACTIVE\nUNKNOWN_STATUS\n";
        let rep_invalid = validate_csv_content(&schema, csv_invalid);
        assert!(!rep_invalid.passed);
        assert_eq!(rep_invalid.invalid_rows, 1);
        assert!(rep_invalid.drifts.iter().any(|d| matches!(
            &d.drift,
            SchemaDriftKind::ValueNotAllowed { column, value, .. }
                if column == "status" && value == "UNKNOWN_STATUS"
        )));
    }

    #[test]
    fn test_range_constraint() {
        let schema = TabularSchema::new(
            "cynic.range_test.v1",
            1,
            vec![
                ColumnDefinition::new("score", DataColumnType::Float, true)
                    .with_range(Some(1.0), Some(5.0)),
            ],
        );

        let csv_valid = "score\n1.0\n3.5\n5.0\n";
        let rep_valid = validate_csv_content(&schema, csv_valid);
        assert!(rep_valid.passed);
        assert_eq!(rep_valid.valid_rows, 3);

        let csv_invalid = "score\n0.5\n5.5\n";
        let rep_invalid = validate_csv_content(&schema, csv_invalid);
        assert!(!rep_invalid.passed);
        assert_eq!(rep_invalid.invalid_rows, 2);
        assert_eq!(rep_invalid.drifts.len(), 2);
        assert!(rep_invalid.drifts.iter().all(|d| matches!(
            &d.drift,
            SchemaDriftKind::ValueOutOfRange { column, .. } if column == "score"
        )));
    }

    #[test]
    fn test_pattern_regex_constraint() {
        let schema = TabularSchema::new(
            "cynic.pattern_test.v1",
            1,
            vec![
                ColumnDefinition::new("asset_code", DataColumnType::Text, true)
                    .with_pattern(r"^AST-\d{4}$"),
            ],
        );

        let csv_valid = "asset_code\nAST-1234\nAST-9999\n";
        let rep_valid = validate_csv_content(&schema, csv_valid);
        assert!(rep_valid.passed);
        assert_eq!(rep_valid.valid_rows, 2);

        let csv_invalid = "asset_code\nAST-1234\nAST-BAD\nNOT-AN-AST\n";
        let rep_invalid = validate_csv_content(&schema, csv_invalid);
        assert!(!rep_invalid.passed);
        assert_eq!(rep_invalid.valid_rows, 1);
        assert_eq!(rep_invalid.invalid_rows, 2);
        assert!(rep_invalid.drifts.iter().any(|d| matches!(
            &d.drift,
            SchemaDriftKind::PatternMismatch { column, value, .. }
                if column == "asset_code" && value == "AST-BAD"
        )));
    }

    #[test]
    fn test_compiled_schema_batch_validation() {
        let schema = TabularSchema::new(
            "cynic.compiled_test.v1",
            1,
            vec![
                ColumnDefinition::new("id", DataColumnType::Text, true)
                    .with_pattern(r"^ID-\d{3}$"),
                ColumnDefinition::new("val", DataColumnType::Integer, true)
                    .with_range(Some(10.0), Some(20.0)),
            ],
        );

        let compiled = CompiledTabularSchema::compile(&schema).expect("compile schema");
        assert_eq!(compiled.columns.len(), 2);
        assert!(compiled.columns[0].compiled_regex.is_some());
        assert!(compiled.columns[1].compiled_regex.is_none());

        let csv = "id,val\nID-100,15\nID-200,18\n";
        let rep = validate_csv_with_compiled(&compiled, csv);
        assert!(rep.passed);
        assert_eq!(rep.valid_rows, 2);

        let csv_bad = "id,val\nBAD-ID,25\n";
        let rep_bad = validate_csv_with_compiled(&compiled, csv_bad);
        assert!(!rep_bad.passed);
        assert_eq!(rep_bad.invalid_rows, 1);
        assert_eq!(rep_bad.drifts.len(), 2);
    }

    #[test]
    fn test_session_summary_schema_and_session_validation() {
        let schema = session_summary_schema();
        assert_eq!(schema.schema_id, "cynic.session_summary.v1");

        let s1 = NormalizedSession {
            session_uid: "sess_123".to_string(),
            harness: "hermes".to_string(),
            source_ref: "hermes_union.sqlite".to_string(),
            turns: vec![],
            tools: vec![],
            reasoning: vec![],
            edges: vec![],
            states: vec![],
            human: vec![],
            native_meta: std::collections::HashMap::new(),
        };

        let report = validate_sessions(&schema, &[s1]);
        assert!(report.passed);
        assert_eq!(report.total_rows, 1);
        assert_eq!(report.valid_rows, 1);
        assert_eq!(report.invalid_rows, 0);

        // Invalid harness drift test
        let s2 = NormalizedSession {
            session_uid: "sess_456".to_string(),
            harness: "unknown_harness_xyz".to_string(),
            source_ref: "dummy.json".to_string(),
            turns: vec![],
            tools: vec![],
            reasoning: vec![],
            edges: vec![],
            states: vec![],
            human: vec![],
            native_meta: std::collections::HashMap::new(),
        };

        let report2 = validate_sessions(&schema, &[s2]);
        assert!(!report2.passed);
        assert_eq!(report2.invalid_rows, 1);
        assert_eq!(report2.drifts.len(), 1);
    }

    #[test]
    fn test_document_chunk_and_claim_schema_validation() {
        let chunk_schema = document_chunk_schema();
        assert_eq!(chunk_schema.schema_id, "cynic.document_chunk.v1");

        let valid_chunk_json = r#"[
            {
                "chunk_id": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "source_path": "docs/foundation/CONSTITUTION.md",
                "line_start": 1,
                "line_end": 45,
                "char_length": 1250,
                "authority": "L0Constitution"
            }
        ]"#;

        let rep_chunk = validate_json_content(&chunk_schema, valid_chunk_json);
        assert!(rep_chunk.passed);
        assert_eq!(rep_chunk.valid_rows, 1);

        let claim_schema = epistemic_claim_schema();
        assert_eq!(claim_schema.schema_id, "cynic.epistemic_claim.v1");

        let valid_claim_json = r#"[
            {
                "id": "claim:zero_io_tabular",
                "category": "architecture",
                "statement": "Tabular validation core is 100% zero-I/O",
                "status": "demonstrated",
                "evidence": "crates/cynic-traces/src/tabular.rs",
                "verify_command": "cargo test -p cynic-traces --lib"
            }
        ]"#;

        let rep_claim = validate_json_content(&claim_schema, valid_claim_json);
        assert!(rep_claim.passed);
        assert_eq!(rep_claim.valid_rows, 1);

        // Invalid claim status drift test
        let invalid_claim_json = r#"[
            {
                "id": "claim:invalid_status",
                "category": "architecture",
                "statement": "Invalid status test",
                "status": "not_a_real_status",
                "evidence": "none"
            }
        ]"#;

        let rep_bad = validate_json_content(&claim_schema, invalid_claim_json);
        assert!(!rep_bad.passed);
        assert_eq!(rep_bad.invalid_rows, 1);

        // Native records builder & validator test
        let r1 = document_chunk_record(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "crates/cynic-traces/src/tabular.rs",
            10,
            50,
            1500,
            "L1Specification",
        );
        let rep_direct = validate_document_chunk_records(&[r1]);
        assert!(rep_direct.passed);
        assert_eq!(rep_direct.valid_rows, 1);

        // Invalid chunk (starvation length < 60 chars)
        let r_starved = document_chunk_record(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            "tiny.md",
            1,
            2,
            25, // < 60
            "L3Operation",
        );
        let rep_starved = validate_document_chunk_records(&[r_starved]);
        assert!(!rep_starved.passed);
        assert_eq!(rep_starved.invalid_rows, 1);

        let c1 = epistemic_claim_record(
            "claim:test",
            "competence",
            "antigravity adheres to zero-hyphen CLI policy",
            "demonstrated",
            "crates/cynic-cli/src/command_manifest.rs",
            Some("cargo test -p cynic-cli --lib command_manifest"),
        );
        let rep_claim_direct = validate_epistemic_claim_records(&[c1]);
        assert!(rep_claim_direct.passed);
        assert_eq!(rep_claim_direct.valid_rows, 1);
    }
}
