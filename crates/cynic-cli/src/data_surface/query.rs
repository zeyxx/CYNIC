//! In-memory SQLite projection query execution for validated tabular datasets.

use std::path::Path;
use std::process::ExitCode;

use cynic_protocol::tabular::TabularSchema;

pub fn execute_query(file_path: &Path, schema_arg: Option<&str>, sql: &str, json: bool) -> ExitCode {
    let schema = match resolve_schema(schema_arg) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {}", e);
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

    let records = match parse_file_to_records(file_path, &content) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error parsing data file '{}': {}", file_path.display(), e);
            return ExitCode::FAILURE;
        }
    };

    // Pre-validation gate: Strict Zero-Defect verification
    let report = cynic_traces::tabular::validate_record_maps(&schema, &records);
    if !report.passed {
        eprintln!(
            "Error: Data validation failed against '{}' ({} invalid row(s)). Query aborted.",
            schema.schema_id,
            report.invalid_rows
        );
        for drift in report.drifts.iter().take(5) {
            eprintln!("  - {:?}", drift.drift);
        }
        return ExitCode::FAILURE;
    }

    // Initialize in-memory SQLite projection
    let mut projection = match cynic_storage::TabularMemoryProjection::create_from_schema(schema, None) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: Failed to initialize SQLite in-memory projection: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if let Err(e) = projection.insert_records(&records) {
        eprintln!("Error: Failed to load records into SQLite: {}", e);
        return ExitCode::FAILURE;
    }

    let query_result = match projection.query(sql) {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Error: Query failed: {}", e);
            return ExitCode::FAILURE;
        }
    };

    if json {
        match serde_json::to_string_pretty(&query_result) {
            Ok(s) => println!("{}", s),
            Err(e) => {
                eprintln!("Error serializing query result to JSON: {}", e);
                return ExitCode::FAILURE;
            }
        }
    } else {
        print_ascii_table(&query_result);
        println!(
            "\n[data query] {} row(s) returned in {}ms (table: '{}' / 'data')",
            query_result.row_count,
            query_result.elapsed_ms,
            projection.table_name()
        );
    }

    ExitCode::SUCCESS
}

pub fn resolve_schema(schema_arg: Option<&str>) -> Result<TabularSchema, String> {
    let arg = match schema_arg {
        Some(a) => a.trim(),
        None => {
            return Err(
                "Missing required schema. Specify `--schema <name|path>`. Canonical options: 'session', 'turn', 'asset', 'natcat', 'chunk', 'claim', or path to a TabularSchema JSON file.".to_string(),
            );
        }
    };

    let p = Path::new(arg);
    if p.exists() && p.is_file() {
        let content = std::fs::read_to_string(p)
            .map_err(|e| format!("Cannot read schema file '{}': {}", p.display(), e))?;
        return serde_json::from_str(&content)
            .map_err(|e| format!("Invalid TabularSchema JSON in '{}': {}", p.display(), e));
    }

    match arg.to_lowercase().as_str() {
        "asset" | "assets" | "node_inventory" | "inventory" => {
            Ok(cynic_traces::tabular::node_inventory_schema())
        }
        "natcat" | "insurance" => Ok(cynic_traces::tabular::natcat_claims_schema()),
        "session" | "sessions" | "session_summary" => {
            Ok(cynic_traces::tabular::session_summary_schema())
        }
        "turn" | "turns" | "session_turn" => Ok(cynic_traces::tabular::session_turn_schema()),
        "chunk" | "chunks" | "document_chunk" => {
            Ok(cynic_traces::tabular::document_chunk_schema())
        }
        "claim" | "claims" | "epistemic_claim" | "fact" => {
            Ok(cynic_traces::tabular::epistemic_claim_schema())
        }
        other => Err(format!(
            "Unknown schema '{}'. Expected canonical name ('session', 'turn', 'asset', 'natcat', 'chunk', 'claim') or path to JSON schema file.",
            other
        )),
    }
}

pub fn parse_file_to_records(
    file_path: &Path,
    content: &str,
) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "csv" {
        let (headers, rows) = cynic_traces::tabular::parse_csv_records(content)?;

        let mut records = Vec::with_capacity(rows.len());
        for row in rows {
            let mut map = serde_json::Map::new();
            for (idx, val) in row.into_iter().enumerate() {
                if let Some(h) = headers.get(idx) {
                    map.insert(h.clone(), serde_json::Value::String(val));
                }
            }
            records.push(map);
        }
        Ok(records)
    } else if ext == "jsonl" || ext == "ndjson" {
        cynic_traces::tabular::parse_jsonl_records(content)
    } else {
        let trimmed = content.trim_start();
        if trimmed.starts_with('[') {
            cynic_traces::tabular::parse_json_records(content)
        } else if trimmed.starts_with('{') {
            cynic_traces::tabular::parse_jsonl_records(content)
        } else {
            let (headers, rows) = cynic_traces::tabular::parse_csv_records(content)?;
            let mut records = Vec::with_capacity(rows.len());
            for row in rows {
                let mut map = serde_json::Map::new();
                for (idx, val) in row.into_iter().enumerate() {
                    if let Some(h) = headers.get(idx) {
                        map.insert(h.clone(), serde_json::Value::String(val));
                    }
                }
                records.push(map);
            }
            Ok(records)
        }
    }
}

fn print_ascii_table(res: &cynic_storage::TabularQueryResult) {
    if res.columns.is_empty() {
        println!("(No columns returned)");
        return;
    }

    // Determine column widths
    let mut widths: Vec<usize> = res.columns.iter().map(|c| c.len()).collect();
    for row in &res.rows {
        for (i, val) in row.iter().enumerate() {
            if i < widths.len() {
                let s = format_val(val);
                if s.len() > widths[i] {
                    widths[i] = s.len().min(60);
                }
            }
        }
    }

    // Border line
    let border = format!(
        "+{}+",
        widths
            .iter()
            .map(|w| "-".repeat(w + 2))
            .collect::<Vec<_>>()
            .join("+")
    );

    println!("{border}");

    // Header
    let header = format!(
        "|{}|",
        res.columns
            .iter()
            .enumerate()
            .map(|(i, col)| {
                let w = widths[i];
                format!(" {:<w$} ", col, w = w)
            })
            .collect::<Vec<_>>()
            .join("|")
    );
    println!("{header}");
    println!("{border}");

    // Rows
    for row in &res.rows {
        let row_str = format!(
            "|{}|",
            row.iter()
                .enumerate()
                .map(|(i, val)| {
                    let w = widths.get(i).copied().unwrap_or(10);
                    let s = format_val(val);
                    let truncated = if s.len() > w {
                        format!("{}...", &s[..w.saturating_sub(3)])
                    } else {
                        s
                    };
                    format!(" {:<w$} ", truncated, w = w)
                })
                .collect::<Vec<_>>()
                .join("|")
        );
        println!("{row_str}");
    }

    println!("{border}");
}

fn format_val(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => val.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_schema_canonical_names() {
        assert_eq!(
            resolve_schema(Some("session")).unwrap().schema_id,
            "cynic.session_summary.v1"
        );
        assert_eq!(
            resolve_schema(Some("turn")).unwrap().schema_id,
            "cynic.session_turn.v1"
        );
        assert_eq!(
            resolve_schema(Some("asset")).unwrap().schema_id,
            "cynic.node_inventory.v1"
        );
        assert_eq!(
            resolve_schema(Some("natcat")).unwrap().schema_id,
            "cynic.natcat_claims.v1"
        );
        assert_eq!(
            resolve_schema(Some("chunk")).unwrap().schema_id,
            "cynic.document_chunk.v1"
        );
        assert_eq!(
            resolve_schema(Some("claim")).unwrap().schema_id,
            "cynic.epistemic_claim.v1"
        );
        assert!(resolve_schema(None).is_err());
        assert!(resolve_schema(Some("unknown_xyz")).is_err());
    }

    #[test]
    fn test_parse_file_to_records_jsonl_and_csv() {
        let csv_content = "node_id,cores,is_active\nnode-1,8,true\nnode-2,16,false";
        let csv_records = parse_file_to_records(Path::new("nodes.csv"), csv_content).unwrap();
        assert_eq!(csv_records.len(), 2);
        assert_eq!(csv_records[0]["node_id"], serde_json::json!("node-1"));
        assert_eq!(csv_records[1]["cores"], serde_json::json!("16"));

        let jsonl_content = "{\"session_id\":\"s-123\",\"total_cost\":0.04}\n{\"session_id\":\"s-456\",\"total_cost\":0.12}";
        let jsonl_records = parse_file_to_records(Path::new("sessions.jsonl"), jsonl_content).unwrap();
        assert_eq!(jsonl_records.len(), 2);
        assert_eq!(jsonl_records[0]["session_id"], serde_json::json!("s-123"));
        assert_eq!(jsonl_records[1]["total_cost"], serde_json::json!(0.12));
    }
}
