//! Tabular data schema contract, drift detection types, and validation reporting.
//!
//! Provides canonical zero-I/O contracts for structured tabular data (CSV, JSONL, Parquet-adjacent)
//! supporting schema enforcement, type validation, and drift reporting across pipelines
//! (e.g. IT Asset Management audits, NatCat claim extractions, event logs).

use serde::{Deserialize, Serialize};
use crate::audit::DataReport;

pub const TABULAR_VALIDATION_REPORT_SCHEMA_V1: &str = "cynic.tabular_validation.v1";

/// Canonical data types for tabular columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataColumnType {
    Text,
    Integer,
    Float,
    Boolean,
    TimestampIso,
    Json,
}

impl std::fmt::Display for DataColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Integer => write!(f, "integer"),
            Self::Float => write!(f, "float"),
            Self::Boolean => write!(f, "boolean"),
            Self::TimestampIso => write!(f, "timestamp_iso"),
            Self::Json => write!(f, "json"),
        }
    }
}

/// Definition of a single column in a tabular schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataColumnType,
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn default_true() -> bool {
    true
}

impl ColumnDefinition {
    pub fn new(name: impl Into<String>, data_type: DataColumnType, required: bool) -> Self {
        Self {
            name: name.into(),
            data_type,
            required,
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Specification of a tabular schema contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularSchema {
    pub schema_id: String,
    pub version: u32,
    pub columns: Vec<ColumnDefinition>,
    /// When true, any column present in input but not defined here is flagged as unexpected drift.
    #[serde(default)]
    pub strict_columns: bool,
}

impl TabularSchema {
    pub fn new(schema_id: impl Into<String>, version: u32, columns: Vec<ColumnDefinition>) -> Self {
        Self {
            schema_id: schema_id.into(),
            version,
            columns,
            strict_columns: false,
        }
    }

    pub fn with_strict_columns(mut self, strict: bool) -> Self {
        self.strict_columns = strict;
        self
    }

    pub fn get_column(&self, name: &str) -> Option<&ColumnDefinition> {
        self.columns.iter().find(|c| c.name == name)
    }

    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }
}

/// Severity of a detected schema drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftSeverity {
    Warning,
    Critical,
}

/// Specific category of schema or data drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SchemaDriftKind {
    /// A required column defined in the schema is missing in the dataset or row.
    ColumnMissing { column: String },
    /// An unexpected column is present in strict mode.
    UnexpectedColumn { column: String },
    /// Cell value cannot be parsed as the declared type.
    TypeMismatch {
        column: String,
        expected: String,
        found_sample: String,
    },
    /// A cell is null or empty in a required column.
    NullViolation { column: String },
    /// Structural row violation (e.g. invalid CSV delimiter, malformed JSON).
    MalformedRecord { message: String },
}

/// Recorded instance of schema drift during tabular validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaDrift {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub row_index: Option<usize>,
    pub severity: DriftSeverity,
    pub drift: SchemaDriftKind,
}

/// Comprehensive report of tabular validation against a schema contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabularValidationReport {
    pub schema: String,
    pub target_schema_id: String,
    pub target_schema_version: u32,
    pub total_rows: usize,
    pub valid_rows: usize,
    pub invalid_rows: usize,
    pub passed: bool,
    pub drifts: Vec<SchemaDrift>,
}

impl TabularValidationReport {
    pub fn new(target_schema_id: impl Into<String>, target_schema_version: u32) -> Self {
        Self {
            schema: TABULAR_VALIDATION_REPORT_SCHEMA_V1.to_string(),
            target_schema_id: target_schema_id.into(),
            target_schema_version,
            total_rows: 0,
            valid_rows: 0,
            invalid_rows: 0,
            passed: true,
            drifts: Vec::new(),
        }
    }

    pub fn record_drift(&mut self, drift: SchemaDrift) {
        if drift.severity == DriftSeverity::Critical {
            self.passed = false;
        }
        self.drifts.push(drift);
    }
}

impl DataReport for TabularValidationReport {
    fn schema_id(&self) -> &'static str {
        TABULAR_VALIDATION_REPORT_SCHEMA_V1
    }

    fn to_export_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tabular_schema_definition_and_roundtrip() {
        let schema = TabularSchema::new(
            "cynic.it_assets.v1",
            1,
            vec![
                ColumnDefinition::new("asset_id", DataColumnType::Text, true)
                    .with_description("Unique identifier of the IT asset"),
                ColumnDefinition::new("hostname", DataColumnType::Text, true),
                ColumnDefinition::new("ip_address", DataColumnType::Text, false),
                ColumnDefinition::new("criticality_score", DataColumnType::Integer, true),
                ColumnDefinition::new("compliant", DataColumnType::Boolean, true),
                ColumnDefinition::new("last_scan", DataColumnType::TimestampIso, false),
            ],
        )
        .with_strict_columns(true);

        assert_eq!(schema.column_names().len(), 6);
        assert!(schema.get_column("hostname").is_some());
        assert!(schema.get_column("nonexistent").is_none());

        let json = serde_json::to_string_pretty(&schema).expect("serialize schema");
        let deserialized: TabularSchema = serde_json::from_str(&json).expect("deserialize schema");
        assert_eq!(schema, deserialized);
    }

    #[test]
    fn test_validation_report_datareport_contract() {
        let mut report = TabularValidationReport::new("cynic.natcat_claims.v1", 1);
        report.total_rows = 10;
        report.valid_rows = 9;
        report.invalid_rows = 1;
        report.record_drift(SchemaDrift {
            row_index: Some(4),
            severity: DriftSeverity::Critical,
            drift: SchemaDriftKind::TypeMismatch {
                column: "claim_amount".to_string(),
                expected: "float".to_string(),
                found_sample: "NaN_OR_INVALID".to_string(),
            },
        });

        assert!(!report.passed);
        assert_eq!(report.schema_id(), TABULAR_VALIDATION_REPORT_SCHEMA_V1);

        let json = report.to_export_json().expect("export json");
        let parsed: TabularValidationReport = serde_json::from_str(&json).expect("parse json");
        assert_eq!(report, parsed);
    }
}
