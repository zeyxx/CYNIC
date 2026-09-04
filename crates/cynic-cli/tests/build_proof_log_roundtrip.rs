//! Round-trip for the operator proof log: append, filter, limit, and the
//! missing-file and corrupt-line contracts.

use cynic_cli::build_proof_log::{append_proof, read_proofs};
use std::sync::atomic::{AtomicU64, Ordering};

static PROOF_LOG_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_log() -> String {
    let id = PROOF_LOG_SEQ.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cynic-proof-log-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create proof log dir");
    dir.join("proofs.jsonl").display().to_string()
}

fn proof(commit: &str) -> String {
    serde_json::json!({
        "schema": "cynic.pipeline_build_proof.v1",
        "node": "cynic-builder-ssh",
        "source_commit": commit,
        "action": "cargo:check",
        "status": "converged",
    })
    .to_string()
}

#[test]
fn append_then_filter_by_commit_with_limit() {
    let log = temp_log();
    append_proof(&log, &proof(&"a".repeat(40))).expect("append first");
    append_proof(&log, &proof(&"b".repeat(40))).expect("append second");

    let (all, skipped) = read_proofs(&log, None, 0).expect("read all");
    assert!(skipped.is_empty());
    assert_eq!(all.len(), 2);

    let (filtered, _) = read_proofs(&log, Some(&"b".repeat(40)), 0).expect("read filtered");
    assert_eq!(filtered.len(), 1);

    let (limited, _) = read_proofs(&log, None, 1).expect("read limited");
    assert_eq!(limited.len(), 1);
    assert_eq!(
        limited[0].get("source_commit").and_then(|v| v.as_str()),
        Some("b".repeat(40).as_str())
    );
}

#[test]
fn missing_log_reads_as_empty_history() {
    let missing = format!(
        "{}/no-such-dir/proofs.jsonl",
        std::env::temp_dir().display()
    );
    let (proofs, skipped) = read_proofs(&missing, None, 0).expect("read missing");
    assert!(proofs.is_empty());
    assert!(skipped.is_empty());
}

#[test]
fn corrupt_line_is_skipped_loudly() {
    let log = temp_log();
    append_proof(&log, &proof(&"a".repeat(40))).expect("append valid");
    std::fs::create_dir_all(std::path::Path::new(&log).parent().expect("parent"))
        .expect("ensure dir");
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .expect("open log");
        writeln!(file, "not json {{{{").expect("append corrupt line");
    }

    let (proofs, skipped) = read_proofs(&log, None, 0).expect("read with corrupt line");
    assert_eq!(proofs.len(), 1);
    assert_eq!(skipped.len(), 1);
}

#[test]
fn empty_and_invalid_documents_are_refused() {
    let log = temp_log();
    assert!(append_proof(&log, "   ").is_err());
    assert!(append_proof(&log, "{oops").is_err());
    let (proofs, _) = read_proofs(&log, None, 0).expect("read");
    assert!(proofs.is_empty());
}
