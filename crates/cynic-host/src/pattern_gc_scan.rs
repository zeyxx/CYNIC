//! JSONL pattern scanner feeding `cynic_protocol::pattern_gc`.
//!
//! Streaming observation half of the pattern GC: walks session JSONL files,
//! extracts top-level RFC3339 timestamps and error-bucket signatures, then
//! windows events through the pure `pattern_gc::scan` contract. Decision
//! logic stays in `cynic-protocol`; this adapter only observes.
//!
//! Counting rule mirrors `trace_miner.py` (the source of the calibrated
//! `RetentionPolicy::observed_default`): every matching bucket increments —
//! one line can feed several `error_buckets.*` keys.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use chrono::DateTime;
use cynic_protocol::pattern_gc::{
    PatternGcReport, PatternObservation, RetentionPolicy, build_report, window_events,
};

use crate::bucket_signatures;
use cynic_protocol::autoresearch_port::AutoresearchPortError;

/// Schema of the scan projection emitted by this adapter.
pub const SCAN_SCHEMA: &str = "cynic.pattern_gc.scan.v1";

/// One streamed scan result: windowed observations plus honesty counters.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternScanOutcome {
    /// Windowed observations, keys sorted alphabetically (deterministic).
    pub observations: Vec<PatternObservation>,
    /// Boundary passed to the scan (unix seconds).
    pub boundary_s: i64,
    /// Recent-window span passed to the scan (seconds).
    pub recent_span_s: u64,
    /// Total non-empty lines streamed.
    pub lines_scanned: u64,
    /// Lines skipped for lack of a parsable top-level timestamp.
    pub untimestamped_lines: u64,
    /// Events stamped beyond the boundary (surfaced, never folded).
    pub out_of_order_events: u64,
}

impl PatternScanOutcome {
    /// Seal the outcome into a self-digesting report under `policy`.
    pub fn seal(&self, policy: &RetentionPolicy) -> PatternGcReport {
        let assessments = cynic_protocol::pattern_gc::classify_all(&self.observations, policy);
        build_report(
            &assessments,
            policy,
            self.boundary_s,
            self.recent_span_s,
            self.out_of_order_events,
        )
    }
}

/// Extract the top-level `timestamp` field of one JSONL line as unix
/// seconds. Returns `None` for missing or malformed stamps (callers count
/// them, never guess).
pub fn event_timestamp_s(line: &str) -> Option<i64> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    let raw = value.get("timestamp")?.as_str()?;
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp())
}

/// All `error_buckets.<name>` keys matched by one line, mirroring
/// `trace_miner.py`: at most one increment per bucket label — several
/// patterns share a label (`rustc`, `sigsegv` -> `toolchain_ice`), and
/// overlapping signatures feed distinct buckets.
pub fn bucket_keys_for_line(line: &str) -> Vec<String> {
    bucket_signatures::matching_buckets(line)
        .into_iter()
        .map(|domain| format!("error_buckets.{domain}"))
        .collect()
}

/// Stream one reader, collecting per-key timestamps then windowing them.
///
/// Pure with respect to the filesystem: any `BufRead` works, so tests run
/// against in-memory corpora.
pub fn scan_reader<R: BufRead>(
    reader: R,
    boundary_s: i64,
    recent_span_s: u64,
) -> Result<PatternScanOutcome, AutoresearchPortError> {
    // Validate windowing parameters up front: an invalid request must fail
    // even when the input carries no events at all.
    if recent_span_s == 0 {
        return Err(AutoresearchPortError::new(
            "invalid_recent_span",
            "recent_span_s must be positive",
        ));
    }

    // BTreeMap => deterministic key order regardless of insertion order.
    let mut events: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    let mut lines_scanned: u64 = 0;
    let mut untimestamped_lines: u64 = 0;

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue, // malformed transport line: skip, keep streaming
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        lines_scanned += 1;
        let Some(ts) = event_timestamp_s(trimmed) else {
            untimestamped_lines += 1;
            continue;
        };
        for key in bucket_keys_for_line(trimmed) {
            events.entry(key).or_default().push(ts);
        }
    }

    let mut observations = Vec::new();
    let mut out_of_order_total: u64 = 0;
    for (key, stamps) in &events {
        let Some(outcome) = window_events(stamps, boundary_s, recent_span_s) else {
            // Unreachable after the up-front validation; kept as a
            // fail-loud guard rather than a silent normalization.
            return Err(AutoresearchPortError::new(
                "invalid_recent_span",
                format!("recent_span_s must be positive, got {recent_span_s}"),
            ));
        };
        out_of_order_total += outcome.out_of_order;
        observations.push(PatternObservation {
            key: key.clone(),
            windows: outcome.windows,
        });
    }

    Ok(PatternScanOutcome {
        observations,
        boundary_s,
        recent_span_s,
        lines_scanned,
        untimestamped_lines,
        out_of_order_events: out_of_order_total,
    })
}

/// Collect `*.jsonl` paths under `root`, recursively (bounded depth 4).
fn collect_jsonl_paths(root: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_paths(&path, depth + 1, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            out.push(path);
        }
    }
}

/// Scan a directory tree (or single file) of session JSONL traces.
pub fn scan_path(
    root: &Path,
    boundary_s: i64,
    recent_span_s: u64,
) -> Result<PatternScanOutcome, AutoresearchPortError> {
    let mut paths = Vec::new();
    if root.is_file() {
        paths.push(root.to_path_buf());
    } else {
        collect_jsonl_paths(root, 0, &mut paths);
    }
    paths.sort();

    let mut merged: Option<PatternScanOutcome> = None;
    for path in paths {
        let file = File::open(&path).map_err(|e| {
            AutoresearchPortError::new("trace_open_failed", format!("{}: {}", path.display(), e))
        })?;
        let partial = scan_reader(BufReader::new(file), boundary_s, recent_span_s)?;
        merged = Some(match merged {
            None => partial,
            Some(acc) => merge_outcomes(acc, partial),
        });
    }

    merged.ok_or_else(|| {
        AutoresearchPortError::new(
            "corpus_empty",
            format!("no .jsonl traces found under {}", root.display()),
        )
    })
}

/// Fold one file's outcome into an accumulator (sums and appends).
fn merge_outcomes(mut acc: PatternScanOutcome, next: PatternScanOutcome) -> PatternScanOutcome {
    acc.lines_scanned += next.lines_scanned;
    acc.untimestamped_lines += next.untimestamped_lines;
    acc.out_of_order_events += next.out_of_order_events;
    for observation in next.observations {
        if let Some(existing) = acc
            .observations
            .iter_mut()
            .find(|o| o.key == observation.key)
        {
            existing.windows.historical += observation.windows.historical;
            existing.windows.recent += observation.windows.recent;
            existing.windows.historical_span = existing
                .windows
                .historical_span
                .max(observation.windows.historical_span);
        } else {
            acc.observations.push(observation);
        }
    }
    acc.observations.sort_by(|a, b| a.key.cmp(&b.key));
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    const COLD_TS: &str = "2025-12-01T00:00:00.000Z"; // 1764547200
    const HOT_TS: &str = "2026-04-01T00:00:00.000Z"; // 1774972800
    const BOUNDARY: i64 = 1_775_549_722; // 2026-04-07T08:15:22Z
    const SPAN_30D: u64 = 2_592_000;

    fn line_with_ts(ts: &str, text: &str) -> String {
        format!(
            r#"{{"timestamp":"{ts}","type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"{text}"}}]}}}}"#
        )
    }

    #[test]
    fn parses_real_corpus_timestamp_shape() {
        // Verbatim shape observed in the mined corpus (46/50 lines stamped).
        let line = r#"{"timestamp":"2026-03-08T08:15:22.222Z","type":"progress"}"#;
        assert_eq!(event_timestamp_s(line), Some(1_772_957_722));
        assert_eq!(event_timestamp_s(r#"{"type":"user"}"#), None);
        assert_eq!(event_timestamp_s("not json"), None);
        assert_eq!(event_timestamp_s(r#"{"timestamp":"not-a-date"}"#), None);
    }

    #[test]
    fn overlapping_signatures_feed_multiple_buckets_like_the_miner() {
        let line = line_with_ts(COLD_TS, "SIGSEGV then could not compile");
        let keys = bucket_keys_for_line(&line);
        assert!(keys.contains(&"error_buckets.toolchain_ice".to_string()));
        assert!(keys.contains(&"error_buckets.cargo_build".to_string()));
        assert_eq!(bucket_keys_for_line("hello world"), Vec::<String>::new());
    }

    #[test]
    fn scan_reader_windows_and_counts_honestly() {
        let mut corpus = String::new();
        // 100 cold ICE events across December.
        for i in 0..100u32 {
            let day = 1 + (i % 28);
            corpus.push_str(&format!(
                "{}\n",
                format_args!(r#"{{"timestamp":"2025-12-{day:02}T10:00:00.000Z","text":"rustc SIGSEGV #{i}"}}"#)
            ));
        }
        corpus.push('\n'); // blank separator line (skipped, not counted)
        // 60 hot network timeouts, March 10-30 (inside a 30-day hot window:
        // boundary - 30d = 2026-03-08T08:15Z).
        for i in 0..60u32 {
            let day = 10 + (i % 21);
            corpus.push_str(&format!(
                "{}\n",
                format_args!(r#"{{"timestamp":"2026-03-{day:02}T10:00:00.000Z","text":"connection timeout #{i}"}}"#)
            ));
        }
        // One future-stamped line and one unstamped line.
        corpus.push_str(&line_with_ts("2026-05-01T00:00:00.000Z", "timeout later"));
        corpus.push('\n');
        corpus.push_str("{\"text\":\"timeout without stamp\"}\n");

        let outcome = scan_reader(Cursor::new(corpus), BOUNDARY, SPAN_30D).unwrap();

        assert_eq!(outcome.lines_scanned, 162);
        assert_eq!(outcome.untimestamped_lines, 1);
        assert_eq!(outcome.out_of_order_events, 1);

        let ice = outcome
            .observations
            .iter()
            .find(|o| o.key == "error_buckets.toolchain_ice")
            .expect("ice key present");
        assert_eq!(ice.windows.historical, 100);
        assert_eq!(ice.windows.recent, 0);

        let net = outcome
            .observations
            .iter()
            .find(|o| o.key == "error_buckets.network")
            .expect("network key present");
        assert_eq!(net.windows.recent, 60);

        // Keys are alphabetically ordered (deterministic output).
        let mut keys: Vec<_> = outcome.observations.iter().map(|o| o.key.clone()).collect();
        keys.sort();
        let raw_keys: Vec<_> = outcome.observations.iter().map(|o| o.key.clone()).collect();
        assert_eq!(keys, raw_keys);

        // The sealed report classifies the surge as emerging.
        let report = outcome.seal(&RetentionPolicy::observed_default());
        assert_eq!(report.tally.emerging, 1);
        assert_eq!(report.content_sha256.len(), 64);
    }

    #[test]
    fn scan_is_deterministic_across_runs() {
        let corpus = format!(
            "{}\n{}\n",
            line_with_ts(COLD_TS, "rustc SIGSEGV"),
            line_with_ts(HOT_TS, "connection refused")
        );
        let a = scan_reader(Cursor::new(corpus.clone()), BOUNDARY, SPAN_30D).unwrap();
        let b = scan_reader(Cursor::new(corpus), BOUNDARY, SPAN_30D).unwrap();
        assert_eq!(
            a.seal(&RetentionPolicy::observed_default()).content_sha256,
            b.seal(&RetPolicyAlias::observed_default()).content_sha256
        );
    }

    // Local alias only to keep the determinism test line readable.
    use RetentionPolicy as RetPolicyAlias;

    #[test]
    fn zero_span_request_fails_explicitly() {
        let err = scan_reader(Cursor::new("{}\n"), BOUNDARY, 0).unwrap_err();
        assert_eq!(err.code, "invalid_recent_span");
    }

    #[test]
    fn scans_a_tmp_directory_tree() {
        let tmp = std::env::temp_dir().join(format!("cynic_pgc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("projects/demo")).unwrap();
        std::fs::write(
            tmp.join("projects/demo/a.jsonl"),
            format!("{}\n", line_with_ts(HOT_TS, "permission denied")),
        )
        .unwrap();
        std::fs::write(
            tmp.join("projects/demo/b.jsonl"),
            format!(
                "{}\n{}\n",
                line_with_ts(COLD_TS, "rustc SIGSEGV"),
                line_with_ts(COLD_TS, "rustc SIGSEGV")
            ),
        )
        .unwrap();

        let outcome = scan_path(&tmp, BOUNDARY, SPAN_30D).unwrap();
        assert_eq!(outcome.lines_scanned, 3);
        assert_eq!(outcome.observations.len(), 2);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn empty_corpus_is_an_explicit_error() {
        let tmp = std::env::temp_dir().join(format!("cynic_pgc_empty_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let err = scan_path(&tmp, BOUNDARY, SPAN_30D).unwrap_err();
        assert_eq!(err.code, "corpus_empty");
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn adapter_types_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<PatternScanOutcome>();
    }
}
