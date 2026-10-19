//! Fire-and-forget agent coordination log.
//!
//! Agents append debt sightings, blocks, and decisions as single JSON lines
//! without waiting for anyone; readers converge by reading. No locks, no
//! kernel, no network: `O_APPEND` writes are atomic per line, and a missing
//! log reads as empty history. Corrupt lines are skipped loudly so one bad
//! line cannot hide the rest. Cross-machine sharing is a mounted path away
//! (`CYNIC_COORDINATION_LOG`); until then each machine owns its own log and
//! says so.

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};

use serde::{Deserialize, Serialize};

pub const COORDINATION_ENTRY_SCHEMA: &str = "cynic.coordination_entry.v1";

/// One fire-and-forget entry. `kind` is a free string by design (`debt`,
/// `info`, `block`, `decision` are conventions, not an enum) so new kinds
/// emerge without a schema migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationEntry {
    pub schema: String,
    pub ts_unix_s: u64,
    pub agent: String,
    pub kind: String,
    pub body: String,
    #[serde(default)]
    pub refs: Vec<String>,
}

impl CoordinationEntry {
    pub fn new(agent: &str, kind: &str, body: &str, refs: Vec<String>, now_unix_s: u64) -> Self {
        Self {
            schema: COORDINATION_ENTRY_SCHEMA.to_string(),
            ts_unix_s: now_unix_s,
            agent: agent.to_string(),
            kind: kind.to_string(),
            body: body.to_string(),
            refs,
        }
    }
}

/// Resolve the log path: explicit `CYNIC_COORDINATION_LOG`, else the
/// operator-local share dir. Never hardcodes a host.
pub fn default_coordination_log_path() -> Result<String, String> {
    if let Ok(explicit) = std::env::var("CYNIC_COORDINATION_LOG") {
        if !explicit.trim().is_empty() {
            return Ok(explicit);
        }
    }
    std::env::var("HOME")
        .map(|home| format!("{home}/.local/share/cynic/coordination.jsonl"))
        .map_err(|_| {
            "coordination log path unresolvable: HOME is unset and CYNIC_COORDINATION_LOG is empty".to_string()
        })
}

/// Append one entry as a single line. Fire-and-forget: no locking, no
/// read-modify-write. Refuses empty bodies and non-entries.
pub fn append_entry(log_path: &str, entry: &CoordinationEntry) -> Result<(), String> {
    if entry.body.trim().is_empty() {
        return Err("refusing to append an entry with an empty body".to_string());
    }
    if entry.agent.trim().is_empty() {
        return Err("refusing to append an entry with an empty agent".to_string());
    }
    if entry.kind.trim().is_empty() {
        return Err("refusing to append an entry with an empty kind".to_string());
    }
    let line = serde_json::to_string(entry)
        .map_err(|error| format!("refusing to append an unserializable entry: {error}"))?;
    if let Some(parent) = std::path::Path::new(log_path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                format!("cannot create coordination log dir {}: {error}", parent.display())
            })?;
        }
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|error| format!("cannot open coordination log {log_path}: {error}"))?;
    writeln!(file, "{line}")
        .map_err(|error| format!("cannot write coordination log {log_path}: {error}"))?;
    Ok(())
}

/// Read entries, oldest first. A missing file reads as empty history.
/// Corrupt lines are counted and reported via `skipped_lines`, never fatal.
pub fn read_entries(
    log_path: &str,
    kind_filter: Option<&str>,
    limit: usize,
) -> (Vec<CoordinationEntry>, usize) {
    let file = match std::fs::File::open(log_path) {
        Ok(file) => file,
        Err(_) => return (vec![], 0),
    };
    let mut entries = Vec::new();
    let mut skipped = 0usize;
    for line in BufReader::new(file).lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                skipped += 1;
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CoordinationEntry>(&line) {
            Ok(entry) => {
                if kind_filter.is_some_and(|k| entry.kind != k) {
                    continue;
                }
                entries.push(entry);
            }
            Err(_) => skipped += 1,
        }
    }
    if limit > 0 && entries.len() > limit {
        entries.drain(..entries.len() - limit);
    }
    (entries, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_log(name: &str) -> String {
        let path = std::env::temp_dir().join(format!(
            "cynic-coordination-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        path.to_str().unwrap().to_string()
    }

    #[test]
    fn append_then_read_roundtrips() {
        let log = tmp_log("roundtrip");
        let entry = CoordinationEntry::new(
            "tester",
            "debt",
            "D0 example debt",
            vec!["PR-1".to_string()],
            1_700_000_000,
        );
        append_entry(&log, &entry).expect("append");
        let (entries, skipped) = read_entries(&log, None, 0);
        assert_eq!(skipped, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0], entry);
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn missing_log_reads_as_empty_history() {
        let (entries, skipped) = read_entries("/nonexistent/cynic-coord.jsonl", None, 0);
        assert!(entries.is_empty());
        assert_eq!(skipped, 0);
    }

    #[test]
    fn corrupt_lines_are_skipped_loudly() {
        let log = tmp_log("corrupt");
        std::fs::write(&log, "not json\n{\"also\": \"not an entry\"}\n").expect("seed");
        append_entry(
            &log,
            &CoordinationEntry::new("tester", "info", "good line", vec![], 1),
        )
        .expect("append");
        let (entries, skipped) = read_entries(&log, None, 0);
        assert_eq!(entries.len(), 1);
        assert_eq!(skipped, 2);
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn kind_filter_and_limit_hold() {
        let log = tmp_log("filter");
        for (i, kind) in ["debt", "info", "debt"].iter().enumerate() {
            append_entry(
                &log,
                &CoordinationEntry::new("tester", kind, &format!("body {i}"), vec![], i as u64),
            )
            .expect("append");
        }
        let (debts, _) = read_entries(&log, Some("debt"), 0);
        assert_eq!(debts.len(), 2);
        let (last_one, _) = read_entries(&log, None, 1);
        assert_eq!(last_one.len(), 1);
        assert_eq!(last_one[0].body, "body 2");
        let _ = std::fs::remove_file(&log);
    }

    #[test]
    fn empty_body_agent_or_kind_is_refused() {
        let log = tmp_log("refused");
        for (agent, kind, body) in [("", "debt", "x"), ("a", "", "x"), ("a", "debt", "  ")] {
            let entry = CoordinationEntry::new(agent, kind, body, vec![], 1);
            assert!(append_entry(&log, &entry).is_err());
        }
        assert!(!std::path::Path::new(&log).exists());
    }
}
