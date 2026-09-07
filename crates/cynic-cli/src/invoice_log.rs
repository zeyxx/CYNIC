//! Operator-side retention for metabolic invoices.
//!
//! Invoices are evidence, not stdout decor: every invoice computed by
//! `cynic build invoice` is appended as one JSON line to a durable log the
//! operator owns. Only documents carrying the invoice schema are retained.
//! A missing log file reads as empty history, never as an error.

use std::fs::OpenOptions;
use std::io::Write;

/// Schema accepted for retention.
pub const METABOLIC_INVOICE_SCHEMA: &str = "cynic.workload.metabolic_invoice.v1";

/// Resolve the default invoice log path from the operator home.
pub fn default_invoice_log_path() -> Result<String, String> {
    std::env::var("HOME")
        .map(|home| format!("{home}/.local/share/cynic/metabolic-invoices.jsonl"))
        .map_err(|_| {
            "invoice log path unresolvable: HOME is unset and no --invoice-log was given"
                .to_string()
        })
}

/// Append one invoice JSON document as a single line. Parent directories are
/// created; the document must be valid JSON carrying the invoice schema.
pub fn append_invoice(log_path: &str, invoice_json: &str) -> Result<(), String> {
    let trimmed = invoice_json.trim();
    if trimmed.is_empty() {
        return Err("refusing to append an empty invoice document".to_string());
    }
    let value: serde_json::Value = serde_json::from_str(trimmed)
        .map_err(|error| format!("refusing to append invalid invoice JSON: {error}"))?;
    if value.get("schema").and_then(|v| v.as_str()) != Some(METABOLIC_INVOICE_SCHEMA) {
        return Err(format!(
            "refusing to append non-invoice document (schema must be {METABOLIC_INVOICE_SCHEMA})"
        ));
    }
    if let Some(parent) = std::path::Path::new(log_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "cannot create invoice log dir {}: {error}",
                    parent.display()
                )
            })?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("cannot open invoice log {log_path}: {error}"))?;
    writeln!(file, "{trimmed}")
        .map_err(|error| format!("cannot write invoice log {log_path}: {error}"))?;
    Ok(())
}

/// Read retained invoices, newest last. Filters by exact source commit when
/// given and keeps at most `limit` entries (`0` means no limit).
pub fn read_invoices(
    log_path: &str,
    commit: Option<&str>,
    limit: usize,
) -> Result<(Vec<serde_json::Value>, Vec<String>), String> {
    let raw = match std::fs::read_to_string(log_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()));
        }
        Err(error) => return Err(format!("cannot read invoice log {log_path}: {error}")),
    };
    let mut invoices = Vec::new();
    let mut skipped = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Ok(value) => {
                let matches = commit.is_none_or(|wanted| {
                    value
                        .get("source_commit")
                        .and_then(serde_json::Value::as_str)
                        == Some(wanted)
                });
                if matches {
                    invoices.push(value);
                }
            }
            Err(_) => skipped.push(format!("invoice log line {} is not valid JSON", index + 1)),
        }
    }
    if limit > 0 && invoices.len() > limit {
        invoices = invoices.split_off(invoices.len() - limit);
    }
    Ok((invoices, skipped))
}
