//! T3 exit gate: Rust output == Python output on the frozen capture.
//!
//! Fixtures in `tests/parity/` are the Python normalizer's canonical JSON
//! for 7 sessions (plain, codex-reasoning, compaction, corrections) plus
//! aggregate stats for 200 sessions. The Rust normalizer must reproduce
//! them exactly. Skips when the frozen capture is absent.

use std::collections::HashMap;
use std::path::PathBuf;

use cynic_traces::adapters::hermes::HermesNormalizer;
use cynic_traces::ports::NormalizerPort;

const SOURCE: &str = "/var/lib/cynic/vault/corpus/captures/hermes_union_20261019.sqlite";

fn source_path() -> String {
    std::env::var("CYNIC_TRACES_SOURCE").unwrap_or_else(|_| SOURCE.into())
}

fn parity_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/parity")
}

fn floats_equal(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    if a.is_nan() || b.is_nan() || a.is_infinite() || b.is_infinite() {
        return false;
    }
    (a - b).abs() <= 1e-9 * 1.0f64.max(a.abs()).max(b.abs())
}

/// Semantic equality: maps order-insensitive, vecs ordered, floats compared
/// with epsilon (Python repr and serde_json print the same f64 differently —
/// e.g. telemetry timestamps — while integers must match exactly).
fn values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    match (a, b) {
        (serde_json::Value::Number(x), serde_json::Value::Number(y)) => {
            match (x.as_i64(), y.as_i64()) {
                (Some(xi), Some(yi)) => xi == yi,
                _ => match (x.as_f64(), y.as_f64()) {
                    (Some(xf), Some(yf)) => floats_equal(xf, yf),
                    _ => false,
                },
            }
        }
        (serde_json::Value::Array(va), serde_json::Value::Array(vb)) => {
            va.len() == vb.len() && va.iter().zip(vb.iter()).all(|(x, y)| values_equal(x, y))
        }
        (serde_json::Value::Object(ma), serde_json::Value::Object(mb)) => {
            ma.len() == mb.len()
                && ma
                    .iter()
                    .all(|(k, x)| mb.get(k).is_some_and(|y| values_equal(x, y)))
        }
        _ => a == b,
    }
}

fn first_diffs(
    path: &str,
    a: &serde_json::Value,
    b: &serde_json::Value,
    out: &mut Vec<String>,
) {
    if out.len() >= 5 || a == b {
        return;
    }
    match (a, b) {
        (serde_json::Value::Object(ma), serde_json::Value::Object(mb)) => {
            let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let child = format!("{path}.{k}");
                match (ma.get(k), mb.get(k)) {
                    (Some(x), Some(y)) => {
                        if !values_equal(x, y) {
                            first_diffs(&child, x, y, out);
                        }
                    }
                    (Some(x), None) => out.push(format!("{child}: rust-only {x}")),
                    (None, Some(y)) => out.push(format!("{child}: python-only {y}")),
                    (None, None) => {}
                }
                if out.len() >= 5 {
                    return;
                }
            }
        }
        (serde_json::Value::Array(va), serde_json::Value::Array(vb)) => {
            if va.len() != vb.len() {
                out.push(format!(
                    "{path}: len rust={} python={}",
                    va.len(),
                    vb.len()
                ));
                return;
            }
            for (i, (x, y)) in va.iter().zip(vb.iter()).enumerate() {
                if values_equal(x, y) {
                    continue;
                }
                first_diffs(&format!("{path}[{i}]"), x, y, out);
                if out.len() >= 5 {
                    return;
                }
            }
        }
        _ => out.push(format!("{path}: rust={a} python={b}")),
    }
}

#[test]
fn rust_matches_python_session_json() {
    let source = source_path();
    if !PathBuf::from(&source).exists() {
        eprintln!("frozen capture absent, skipping parity");
        return;
    }
    let dir = parity_dir();
    let manifest = std::fs::read_to_string(dir.join("sessions.txt")).unwrap();
    let normalizer = HermesNormalizer::new(&source);
    for line in manifest.lines() {
        let (session_ref, file) = line.split_once('=').unwrap();
        let expected: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join(file)).unwrap()).unwrap();
        let session = normalizer
            .normalize(session_ref)
            .unwrap_or_else(|e| panic!("{session_ref}: rust normalize failed: {e}"));
        let actual = serde_json::to_value(&session).unwrap();
        if !values_equal(&actual, &expected) {
            let mut diffs = Vec::new();
            first_diffs(session_ref, &actual, &expected, &mut diffs);
            panic!("parity break on {session_ref}:\n{}", diffs.join("\n"));
        }
    }
}

#[test]
fn rust_matches_python_stats_on_200_sessions() {
    let source = source_path();
    if !PathBuf::from(&source).exists() {
        eprintln!("frozen capture absent, skipping parity");
        return;
    }
    let dir = parity_dir();
    let expected: HashMap<String, HashMap<String, serde_json::Value>> =
        serde_json::from_str(&std::fs::read_to_string(dir.join("stats200.json")).unwrap())
            .unwrap();
    let normalizer = HermesNormalizer::new(&source);
    let mut checked = 0;
    for (session_ref, want) in &expected {
        if want.contains_key("error") {
            assert!(
                normalizer.normalize(session_ref).is_err(),
                "{session_ref}: python errored, rust must error too"
            );
            continue;
        }
        let s = normalizer
            .normalize(session_ref)
            .unwrap_or_else(|e| panic!("{session_ref}: rust normalize failed: {e}"));
        let got = s.stats();
        let num = |k: &str| want[k].as_u64().unwrap() as usize;
        assert_eq!(got.turns, num("turns"), "{session_ref} turns");
        assert_eq!(got.tools, num("tools"), "{session_ref} tools");
        assert_eq!(got.reasoning, num("reasoning"), "{session_ref} reasoning");
        assert_eq!(got.edges, num("edges"), "{session_ref} edges");
        assert_eq!(got.states, num("states"), "{session_ref} states");
        assert_eq!(got.human, num("human"), "{session_ref} human");
        checked += 1;
    }
    assert_eq!(checked, 200, "stats200 must cover 200 sessions");
}
