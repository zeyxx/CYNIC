//! Round-trip for the metabolic invoice journal: append, filter, limit,
//! schema refusal, and the missing-file and corrupt-line contracts.

use cynic_cli::invoice_log::{append_invoice, read_invoices};
use std::sync::atomic::{AtomicU64, Ordering};

static INVOICE_LOG_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_log() -> String {
    let id = INVOICE_LOG_SEQ.fetch_add(1, Ordering::SeqCst);
    let dir = std::env::temp_dir().join(format!("cynic-invoice-log-{}-{id}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create invoice log dir");
    dir.join("invoices.jsonl").display().to_string()
}

fn invoice(commit: &str, wall_ms: u64) -> String {
    serde_json::json!({
        "schema": "cynic.workload.metabolic_invoice.v1",
        "source_commit": commit,
        "proofs_observed": 1,
        "proofs_invoiced": 1,
        "total": {"wall_time_ms": wall_ms},
        "lines": [],
        "residual_unknowns": [],
    })
    .to_string()
}

#[test]
fn append_then_filter_by_commit_with_limit() {
    let log = temp_log();
    append_invoice(&log, &invoice(&"a".repeat(40), 1000)).expect("append first");
    append_invoice(&log, &invoice(&"b".repeat(40), 2000)).expect("append second");

    let (all, skipped) = read_invoices(&log, None, 0).expect("read all");
    assert!(skipped.is_empty());
    assert_eq!(all.len(), 2);

    let (filtered, _) = read_invoices(&log, Some(&"b".repeat(40)), 0).expect("read filtered");
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0]["total"]["wall_time_ms"], 2000);

    let (limited, _) = read_invoices(&log, None, 1).expect("read limited");
    assert_eq!(limited.len(), 1);
}

#[test]
fn non_invoice_documents_are_refused() {
    let log = temp_log();
    assert!(append_invoice(&log, "   ").is_err());
    assert!(append_invoice(&log, "{oops").is_err());
    let proof = serde_json::json!({
        "schema": "cynic.pipeline_build_proof.v1",
        "source_commit": "a".repeat(40),
    })
    .to_string();
    assert!(append_invoice(&log, &proof).is_err());
    let (invoices, _) = read_invoices(&log, None, 0).expect("read");
    assert!(invoices.is_empty());
}

#[test]
fn missing_log_reads_as_empty_history() {
    let missing = format!(
        "{}/no-such-dir/invoices.jsonl",
        std::env::temp_dir().display()
    );
    let (invoices, skipped) = read_invoices(&missing, None, 0).expect("read missing");
    assert!(invoices.is_empty());
    assert!(skipped.is_empty());
}

#[test]
fn corrupt_line_is_skipped_loudly() {
    let log = temp_log();
    append_invoice(&log, &invoice(&"a".repeat(40), 1000)).expect("append valid");
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&log)
            .expect("open log");
        writeln!(file, "not json {{{{").expect("append corrupt line");
    }

    let (invoices, skipped) = read_invoices(&log, None, 0).expect("read with corrupt line");
    assert_eq!(invoices.len(), 1);
    assert_eq!(skipped.len(), 1);
}
