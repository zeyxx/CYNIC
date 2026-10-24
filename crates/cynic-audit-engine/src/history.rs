//! Audit report persistence and history replay (JSON Lines format).

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use cynic_protocol::{AuditReportV2, DataReport};

/// Append an audit report to a persistent history file (JSON Lines format).
///
/// Ensures each execution can be tracked over time for trend analysis,
/// regression tracking, and compliance metrics without altering report payload.
pub fn append_audit_history(report: &AuditReportV2, history_path: &Path) -> Result<(), String> {
    if let Some(parent) = history_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create parent directory for audit history at {:?}: {}",
                    parent, e
                )
            })?;
        }
    }

    let json_line = report.to_export_json()?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path)
        .map_err(|e| format!("failed to open audit history at {:?}: {}", history_path, e))?;

    writeln!(file, "{}", json_line)
        .map_err(|e| format!("failed to write to audit history at {:?}: {}", history_path, e))?;

    Ok(())
}

/// Read all audit reports from a persistent history file (JSON Lines format).
///
/// Returns an empty vector if the file does not exist yet. Skips empty lines
/// and parses each line as a versioned `AuditReportV2`.
pub fn read_audit_history(history_path: &Path) -> Result<Vec<AuditReportV2>, String> {
    if !history_path.exists() {
        return Ok(Vec::new());
    }

    let file = File::open(history_path)
        .map_err(|e| format!("failed to open audit history at {:?}: {}", history_path, e))?;
    let reader = BufReader::new(file);
    let mut reports = Vec::new();

    for (index, line_result) in reader.lines().enumerate() {
        let line = line_result
            .map_err(|e| format!("failed to read line {} from {:?}: {}", index + 1, history_path, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let report = serde_json::from_str::<AuditReportV2>(trimmed).map_err(|e| {
            format!(
                "invalid JSON on line {} in {:?}: {}",
                index + 1,
                history_path,
                e
            )
        })?;
        reports.push(report);
    }

    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::AuditSummary;

    #[test]
    fn test_append_audit_history_roundtrip() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let history_file = temp_dir.path().join("audit_history.jsonl");

        let report = AuditReportV2 {
            schema: cynic_protocol::AUDIT_REPORT_SCHEMA_V2.to_string(),
            generated_at: "2026-10-24T10:00:00Z".to_string(),
            summary: AuditSummary {
                status: "ok".to_string(),
                total_rules_evaluated: 1,
                passed_rules_count: 1,
                critical_count: 0,
                warning_count: 0,
                debt_count: 0,
                residual_unknown_count: 0,
                maturity_score_percent: 100,
            },
            violations: Vec::new(),
            evaluated_rules: Vec::new(),
            residual_unknowns: Vec::new(),
        };

        append_audit_history(&report, &history_file).expect("append 1");
        append_audit_history(&report, &history_file).expect("append 2");

        let contents = std::fs::read_to_string(&history_file).expect("read");
        let lines: Vec<&str> = contents.trim().split('\n').collect();
        assert_eq!(lines.len(), 2);

        let parsed: AuditReportV2 = serde_json::from_str(lines[0]).expect("deserialize line 1");
        assert_eq!(parsed.summary.status, "ok");

        let read_back = read_audit_history(&history_file).expect("read_audit_history");
        assert_eq!(read_back.len(), 2);
        assert_eq!(read_back[0].summary.status, "ok");
    }

    #[test]
    fn test_read_audit_history_nonexistent_returns_empty() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let history_file = temp_dir.path().join("does_not_exist.jsonl");
        let read_back = read_audit_history(&history_file).expect("read_audit_history");
        assert!(read_back.is_empty());
    }
}
