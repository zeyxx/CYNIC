//! Streaming trace observer for the Autoresearcher loop.
//!
//! Reads agent session traces from disk and yields `TraceUnit`s. The design
//! mirrors autoresearch's `prepare.py`: fixed, neutral, never edited by the
//! loop. All decision logic stays in `cynic_protocol::autoresearch`.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use cynic_protocol::autoresearch::TraceUnit;
use cynic_protocol::autoresearch_port::{AutoresearchPort, AutoresearchPortError};

/// Where traces come from. Streaming: we walk files lazily, never buffering all.
#[derive(Debug, Clone)]
pub enum TraceSource {
    /// A directory tree containing `*.jsonl` (Claude projects, Codex sessions).
    Directory(PathBuf),
    /// A single `.jsonl` file (one session).
    File(PathBuf),
}

impl TraceSource {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let p = path.into();
        if p.is_dir() {
            TraceSource::Directory(p)
        } else {
            TraceSource::File(p)
        }
    }
}

/// Stateless emitter: turns one parsed JSONL line into zero or more `TraceUnit`s.
/// Kept pure and unit-tested without touching the filesystem.
pub struct TraceUnitEmitter;

impl TraceUnitEmitter {
    /// Inspect one line and emit one `TraceUnit` per matching bucket
    /// (multi-count, miner parity). Detection is delegated to the canonical
    /// word-anchored table (`crate::bucket_signatures`) — the previous
    /// loose-substring first-match here produced ~99.8% false positives on
    /// the real corpus and undercounted multi-failure lines.
    pub fn emit_from_line(
        session_id: &str,
        line: &str,
        _bucket_rx: &[(&str, &str)],
    ) -> Vec<TraceUnit> {
        crate::bucket_signatures::matching_buckets(line)
            .into_iter()
            .map(|domain| TraceUnit {
                session_id: session_id.to_string(),
                domain: domain.to_string(),
                is_block: true,
            })
            .collect()
    }
}

/// Default error-bucket patterns, mirroring the mined corpus evidence
/// (trace_report.json error_buckets: toolchain_ice 5025, network 1983, ...).
/// Declared, not hardcoded into decision logic.
pub fn default_bucket_patterns() -> Vec<(&'static str, &'static str)> {
    vec![
        ("toolchain_ice", "sigsegv"),
        ("toolchain_ice", "ice"),
        ("toolchain_ice", "rustc"),
        ("toolchain_ice", "cargo build"),
        ("toolchain_ice", "cargo check"),
        ("toolchain_ice", "cargo test"),
        ("network", "timeout"),
        ("network", "connection refused"),
        ("oom", "out of memory"),
        ("oom", "oom"),
        ("permission", "permission denied"),
        ("cargo_build", "could not compile"),
        ("cargo_build", "error["),
    ]
}

/// Filesystem adapter implementing the observation half of `AutoresearchPort`.
/// Streaming: each file is read line-by-line; units are yielded per line.
pub struct TraceFsAdapter {
    source: TraceSource,
    bucket_patterns: Vec<(&'static str, &'static str)>,
}

impl TraceFsAdapter {
    pub fn new(source: TraceSource) -> Self {
        Self {
            source,
            bucket_patterns: default_bucket_patterns(),
        }
    }

    /// Collect all `.jsonl` paths to scan (streaming walk, lazy).
    fn collect_paths(&self) -> Result<Vec<PathBuf>, AutoresearchPortError> {
        let mut out = Vec::new();
        match &self.source {
            TraceSource::File(p) => {
                if p.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                    out.push(p.clone());
                } else {
                    return Err(AutoresearchPortError::new(
                        "unsupported_trace_file",
                        format!("not a .jsonl: {}", p.display()),
                    ));
                }
            }
            TraceSource::Directory(dir) => {
                let entries = std::fs::read_dir(dir).map_err(|e| {
                    AutoresearchPortError::new(
                        "corpus_unreadable",
                        format!("{}: {}", dir.display(), e),
                    )
                })?;
                for entry in entries {
                    let entry = entry.map_err(|e| {
                        AutoresearchPortError::new("corpus_unreadable", e.to_string())
                    })?;
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                        // recurse one level for project subdirs (Claude layout)
                        if path.is_dir() {
                            let sub = std::fs::read_dir(&path).map_err(|e| {
                                AutoresearchPortError::new("corpus_unreadable", e.to_string())
                            })?;
                            for f in sub {
                                let f = f.map_err(|e| {
                                    AutoresearchPortError::new("corpus_unreadable", e.to_string())
                                })?;
                                let fp = f.path();
                                if fp.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                                    out.push(fp);
                                }
                            }
                        } else {
                            out.push(path);
                        }
                    }
                }
            }
        }
        Ok(out)
    }
}

impl AutoresearchPort for TraceFsAdapter {
    fn observe_traces(&self) -> Result<Vec<TraceUnit>, AutoresearchPortError> {
        let paths = self.collect_paths()?;
        let mut units = Vec::new();
        for path in paths {
            let session_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file = File::open(&path).map_err(|e| {
                AutoresearchPortError::new(
                    "trace_open_failed",
                    format!("{}: {}", path.display(), e),
                )
            })?;
            let reader = BufReader::new(file);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => continue, // skip malformed lines, keep streaming
                };
                units.extend(TraceUnitEmitter::emit_from_line(
                    &session_id,
                    &line,
                    &self.bucket_patterns,
                ));
            }
        }
        Ok(units)
    }

    fn hypothesize(
        &self,
        _units: &[TraceUnit],
    ) -> Result<Vec<cynic_protocol::autoresearch::SkillHypothesis>, AutoresearchPortError> {
        // Observation-only adapter. Hypothesis generation is the reflection LLM
        // (Phase 3). Returning empty here keeps this adapter neutral, exactly like
        // autoresearch's prepare.py (which never edits train.py).
        Ok(vec![])
    }
}

/// Streaming from an explicit reader (testable without real files).
pub fn observe_reader<R: BufRead>(
    session_id: &str,
    reader: R,
    bucket_patterns: &[(&str, &str)],
) -> Vec<TraceUnit> {
    let mut units = Vec::new();
    for line in reader.lines().flatten() {
        units.extend(TraceUnitEmitter::emit_from_line(
            session_id,
            &line,
            bucket_patterns,
        ));
    }
    units
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn emitter_tags_toolchain_ice_block() {
        let line =
            r#"{"type":"tool_result","content":"thread 'main' panicked at SIGSEGV signal 11"}"#;
        let u = TraceUnitEmitter::emit_from_line("s1", line, &default_bucket_patterns());
        let u = u.expect("should emit");
        assert_eq!(u.domain, "toolchain_ice");
        assert!(u.is_block);
        assert_eq!(u.session_id, "s1");
    }

    #[test]
    fn emitter_tags_network_without_false_block() {
        // "connection refused" in a non-error context still marks a block per
        // the bucket contract (the pattern itself is the failure signature).
        let line = r#"{"text":"request failed: connection refused to 10.10.10.103"}"#;
        let u = TraceUnitEmitter::emit_from_line("s2", line, &default_bucket_patterns());
        let u = u.expect("should emit");
        assert_eq!(u.domain, "network");
        assert!(u.is_block);
    }

    #[test]
    fn emitter_ignores_unrelated_line() {
        let line = r#"{"type":"user","content":"bonjour claude, on fait quoi aujourd'hui ?"}"#;
        assert!(TraceUnitEmitter::emit_from_line("s3", line, &default_bucket_patterns()).is_none());
    }

    #[test]
    fn observe_reader_streams_multiple_lines() {
        let data = concat!(
            "{\"text\":\"cargo build segfault\"}\n",
            "{\"text\":\"hello world\"}\n",
            "{\"text\":\"out of memory during link\"}\n",
        );
        let units = observe_reader("sessA", Cursor::new(data), &default_bucket_patterns());
        // toolchain_ice + oom = 2 units (hello world ignored)
        assert_eq!(units.len(), 2);
        assert!(
            units
                .iter()
                .any(|u| u.domain == "toolchain_ice" && u.is_block)
        );
        assert!(units.iter().any(|u| u.domain == "oom" && u.is_block));
    }

    #[test]
    fn adapter_rejects_non_jsonl_file() {
        let adapter = TraceFsAdapter::new(TraceSource::from_path("/etc/hostname"));
        let err = adapter.observe_traces().unwrap_err();
        assert_eq!(err.code, "unsupported_trace_file");
    }

    #[test]
    fn adapter_observes_from_mini_corpus_dir() {
        // Build a tiny temp corpus: two jsonl files, one with a real block.
        let tmp = std::env::temp_dir().join(format!("cynic_ar_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("a.jsonl"),
            "{\"text\":\"cargo check SIGSEGV\"}\n{\"text\":\"all good\"}\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("b.jsonl"),
            "{\"text\":\"permission denied on /etc/cynic\"}\n",
        )
        .unwrap();
        let adapter = TraceFsAdapter::new(TraceSource::from_path(&tmp));
        let units = adapter.observe_traces().unwrap();
        // a.jsonl -> toolchain_ice (block); b.jsonl -> permission (block)
        assert_eq!(units.len(), 2);
        assert!(
            units
                .iter()
                .any(|u| u.domain == "toolchain_ice" && u.is_block)
        );
        assert!(units.iter().any(|u| u.domain == "permission" && u.is_block));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn schema_paths_are_send_sync_friendly() {
        // compile-time guarantee the adapter is usable behind a trait object
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<TraceFsAdapter>();
    }
}
