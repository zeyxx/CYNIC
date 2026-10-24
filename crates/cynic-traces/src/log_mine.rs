//! Pure log-line miners and signatures. Evidence references, never raw dumps.
//! Zero I/O: callers pass lines, this classifies and aggregates.
//!
//! Extracted from `cynic-host` as part of the data domain consolidation.

use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

pub const LOG_MINE_REPORT_SCHEMA: &str = "cynic.log_mine_report.v1";

/// One recurrent log signature: normalized pattern + counts + one sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogSignature {
    /// Stable source (`kern`, `journal`, `cynic_trace`).
    pub source: String,
    /// Normalized signature (`oom_kill`, `gpu_page_fault`, `journal_203_exec`).
    pub signature: String,
    /// Occurrence count in the mined window.
    pub count: u64,
    /// One representative raw line (truncated to 512 chars by the probe).
    pub sample: String,
}

/// Mined report for one log bundle: signatures sorted by count desc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogMineReport {
    /// Schema marker for typed argv surfaces.
    pub schema: String,
    /// Total lines scanned.
    pub lines_scanned: u64,
    /// Signatures found, count-desc.
    pub signatures: Vec<LogSignature>,
}

impl cynic_protocol::DataReport for LogMineReport {
    fn schema_id(&self) -> &'static str {
        LOG_MINE_REPORT_SCHEMA
    }

    fn to_export_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }
}

/// Classified count accumulator (internal).
#[derive(Debug, Clone, Default)]
pub struct SignatureCount {
    /// Occurrence count.
    pub count: u64,
    /// First representative sample.
    pub sample: String,
}

/// Classify one line into `(source, signature)` or `None` when uninteresting.
/// Order matters: first match wins, most specific first.
pub fn classify_line(line: &str) -> Option<(&'static str, &'static str)> {
    let lower = line.to_lowercase();
    if lower.contains("out of memory") || lower.contains("oom-killer") || lower.contains("oom_kill")
    {
        return Some(("kern", "oom_kill"));
    }
    if lower.contains("amdgpu") && lower.contains("page fault") {
        return Some(("kern", "gpu_page_fault"));
    }
    if lower.contains("soft lockup") || lower.contains("hung_task") || lower.contains("kcompactd") {
        return Some(("kern", "softlockup"));
    }
    if lower.contains("203/exec") {
        return Some(("journal", "journal_203_exec"));
    }
    if lower.contains("wss") && lower.contains("closed") {
        return Some(("cynic_trace", "events_closed"));
    }
    if lower.contains("401") && lower.contains("unauthorized") {
        return Some(("cynic_trace", "auth_401"));
    }
    if lower.contains("enospc") || lower.contains("no space left") {
        return Some(("kern", "disk_full"));
    }
    None
}

/// Mine lines into a count-desc report. Pure, deterministic.
pub fn mine_log_lines(source_hint: &str, lines: &[&str]) -> LogMineReport {
    let mut acc: BTreeMap<(String, String), SignatureCount> = BTreeMap::new();
    let mut scanned = 0u64;
    for line in lines {
        scanned += 1;
        let (source, signature) = match classify_line(line) {
            Some((s, sig)) => (s.to_string(), sig.to_string()),
            None => continue,
        };
        let _ = source_hint;
        let entry = acc
            .entry((source, signature))
            .or_insert_with(|| SignatureCount {
                count: 0,
                sample: line.chars().take(512).collect(),
            });
        entry.count += 1;
    }

    let mut signatures: Vec<LogSignature> = acc
        .into_iter()
        .map(|((source, signature), sc)| LogSignature {
            source,
            signature,
            count: sc.count,
            sample: sc.sample,
        })
        .collect();

    signatures.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.signature.cmp(&b.signature)));

    LogMineReport {
        schema: LOG_MINE_REPORT_SCHEMA.to_string(),
        lines_scanned: scanned,
        signatures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::DataReport;

    #[test]
    fn test_classify_line_oom() {
        let (source, sig) = classify_line("Kernel: Out of memory: Kill process 1234").expect("classified");
        assert_eq!(source, "kern");
        assert_eq!(sig, "oom_kill");
    }

    #[test]
    fn test_mine_log_lines_ordering_and_data_report() {
        let lines = [
            "random uninteresting line",
            "Out of memory: Kill process 1",
            "Out of memory: Kill process 2",
            "amdgpu page fault detected",
        ];
        let report = mine_log_lines("syslog", &lines);
        assert_eq!(report.lines_scanned, 4);
        assert_eq!(report.signatures.len(), 2);
        assert_eq!(report.signatures[0].signature, "oom_kill");
        assert_eq!(report.signatures[0].count, 2);
        assert_eq!(report.signatures[1].signature, "gpu_page_fault");
        assert_eq!(report.signatures[1].count, 1);

        assert_eq!(report.schema_id(), LOG_MINE_REPORT_SCHEMA);
        let json = report.to_export_json().expect("export json");
        assert!(json.contains("cynic.log_mine_report.v1"));
    }
}
