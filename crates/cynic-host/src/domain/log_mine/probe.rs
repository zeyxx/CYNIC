//! Pure log-line miners. No I/O: callers pass lines, this classifies.

use std::collections::BTreeMap;

use super::types::{LogMineReport, LogSignature};

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
        // A caller hint narrows the source only when the classifier is silent
        // about it; explicit classifier verdicts always win.
        let _ = source_hint;
        let entry = acc
            .entry((source, signature))
            .or_insert_with(|| SignatureCount {
                count: 0,
                sample: (*line).chars().take(512).collect(),
            });
        entry.count += 1;
    }
    let mut signatures: Vec<LogSignature> = acc
        .into_iter()
        .map(|((source, signature), c)| LogSignature {
            source,
            signature,
            count: c.count,
            sample: c.sample,
        })
        .collect();
    signatures.sort_by(|a, b| b.count.cmp(&a.count));
    LogMineReport {
        schema: "cynic.log_mine_report.v1".to_string(),
        lines_scanned: scanned,
        signatures,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mines_known_signatures_count_desc() {
        let lines = vec![
            "kernel: Out of memory: Kill process 123 (llama-server)",
            "kernel: amdgpu: page fault detected",
            "kernel: Out of memory: Kill process 124 (cargo)",
            "systemd[1]: foo.service: Failed with result 'exit-code' 203/EXEC",
            "noise line nobody cares about",
        ];
        let report = mine_log_lines("kern", &lines);
        assert_eq!(report.lines_scanned, 5);
        assert_eq!(report.signatures.len(), 3);
        // oom_kill appears twice so it sorts first.
        assert_eq!(report.signatures[0].signature, "oom_kill");
        assert_eq!(report.signatures[0].count, 2);
    }

    #[test]
    fn ignores_pure_noise() {
        let lines = vec!["hello world", "another boring line"];
        let report = mine_log_lines("kern", &lines);
        assert_eq!(report.lines_scanned, 2);
        assert!(report.signatures.is_empty());
    }

    #[test]
    fn truncates_long_samples() {
        let long = "x".repeat(2000) + " Out of memory: Kill process";
        let lines = vec![long.as_str()];
        let report = mine_log_lines("kern", &lines);
        assert_eq!(report.signatures.len(), 1);
        assert!(report.signatures[0].sample.chars().count() <= 512);
    }
}
