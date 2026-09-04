//! Operator-side retention for build proofs.
//!
//! Trigger proofs are evidence, not stdout decor: every proof is appended as
//! one JSON line to a durable log the operator owns. A missing log file reads
//! as empty history, never as an error. Parse failures on individual lines
//! are skipped loudly in the returned detail so one corrupt line cannot hide
//! the rest.

use std::fs::OpenOptions;
use std::io::Write;

/// Resolve the default proof log path from the operator home.
pub fn default_proof_log_path() -> Result<String, String> {
    std::env::var("HOME")
        .map(|home| format!("{home}/.local/share/cynic/build-proofs.jsonl"))
        .map_err(|_| {
            "proof log path unresolvable: HOME is unset and no --proof-log was given".to_string()
        })
}

/// Append one proof JSON document as a single line. Parent directories are
/// created; the document must be valid JSON.
pub fn append_proof(log_path: &str, proof_json: &str) -> Result<(), String> {
    let trimmed = proof_json.trim();
    if trimmed.is_empty() {
        return Err("refusing to append an empty proof document".to_string());
    }
    serde_json::from_str::<serde_json::Value>(trimmed)
        .map_err(|error| format!("refusing to append invalid proof JSON: {error}"))?;
    if let Some(parent) = std::path::Path::new(log_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("cannot create proof log dir {}: {error}", parent.display())
            })?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("cannot open proof log {log_path}: {error}"))?;
    writeln!(file, "{trimmed}")
        .map_err(|error| format!("cannot write proof log {log_path}: {error}"))?;
    Ok(())
}

/// Read retained proofs, newest last. Filters by exact source commit when
/// given and keeps at most `limit` entries (`0` means no limit).
pub fn read_proofs(
    log_path: &str,
    commit: Option<&str>,
    limit: usize,
) -> Result<(Vec<serde_json::Value>, Vec<String>), String> {
    let raw = match std::fs::read_to_string(log_path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok((Vec::new(), Vec::new()));
        }
        Err(error) => return Err(format!("cannot read proof log {log_path}: {error}")),
    };
    let mut proofs = Vec::new();
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
                    proofs.push(value);
                }
            }
            Err(_) => skipped.push(format!("proof log line {} is not valid JSON", index + 1)),
        }
    }
    if limit > 0 && proofs.len() > limit {
        proofs.drain(..proofs.len() - limit);
    }
    Ok((proofs, skipped))
}
