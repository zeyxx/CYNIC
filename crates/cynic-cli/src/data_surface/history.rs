//! Tabular validation history inspection and offline cryptographic verification.

use std::path::Path;
use std::process::ExitCode;

use cynic_protocol::tabular::TabularValidationReport;

pub fn append_history_report(report: &TabularValidationReport, path: &Path) -> Result<(), String> {
    let json_line = serde_json::to_string(report)
        .map_err(|e| format!("Failed to serialize TabularValidationReport: {}", e))?;
    cynic_host::adapters::jsonl_append::JsonlAppend::append_with_fsync(path, &json_line)
}

pub fn read_history_reports(path: &Path) -> Result<Vec<TabularValidationReport>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open tabular history at {:?}: {}", path, e))?;
    let reader = std::io::BufReader::new(file);
    use std::io::BufRead;
    let mut reports = Vec::new();
    for (idx, line_res) in reader.lines().enumerate() {
        let line = line_res.map_err(|e| format!("IO error reading line {}: {}", idx + 1, e))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let report: TabularValidationReport = serde_json::from_str(trimmed)
            .map_err(|e| format!("JSON decode error at line {}: {}", idx + 1, e))?;
        reports.push(report);
    }
    Ok(reports)
}

pub fn execute_history(limit: usize, json: bool, verify: bool) -> ExitCode {
    if verify {
        let evidence_root = Path::new("evidence");
        let store = match cynic_storage::evidence_journal_file::EvidenceJournalStore::open(evidence_root) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error opening evidence journal at '{}': {}", evidence_root.display(), e);
                return ExitCode::FAILURE;
            }
        };

        let (entries, violations) = store.verify_offline();
        if json {
            let res = serde_json::json!({
                "schema": "cynic.evidence.journal_verification.v1",
                "verified": violations.is_empty(),
                "total_entries": entries.len(),
                "violations_count": violations.len(),
                "violations": violations,
            });
            println!("{}", serde_json::to_string_pretty(&res).unwrap_or_default());
        } else {
            println!("=== CYNIC Cold Evidence Journal Verification ===");
            println!("Journal Root   : {}", evidence_root.display());
            println!("Total Entries  : {}", entries.len());
            println!("Violations     : {}", violations.len());
            println!("Status         : {}", if violations.is_empty() { "PASSED (Cryptographic Hash-Chain Intact)" } else { "FAILED" });
            if !violations.is_empty() {
                println!();
                println!("Integrity Violations:");
                for v in &violations {
                    println!("  - Breach: {}", v);
                }
            }
        }

        if violations.is_empty() {
            return ExitCode::SUCCESS;
        } else {
            return ExitCode::FAILURE;
        }
    }

    let history_path = Path::new("evidence/data_validation_history.jsonl");
    let reports = match read_history_reports(history_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error reading tabular history from '{}': {}", history_path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    if json {
        let display_reports: Vec<&TabularValidationReport> = reports.iter().rev().take(limit).collect();
        match serde_json::to_string_pretty(&display_reports) {
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
